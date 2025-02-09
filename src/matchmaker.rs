//! Match-maker logic, receiving players in the queue.

use std::time::{Duration, Instant};
use std::collections::{hash_map, HashMap};

use rand::Rng;

use crate::tank::{Tank, TankClass};


/// The matchmaker, queuing and sending tanks into battles.
#[derive(Debug)]
pub struct Matchmaker<'reg> {
    /// The maximum tier allowed.
    max_tier: u8,
    /// For each queued tier.
    queue_tiers: HashMap<u8, QueueTier>,
    /// All the tanks in the queue!
    queue_tanks: Vec<QueueTank<'reg>>,
    /// Stores the last wait duration.
    last_wait_duration: Duration,
}

impl<'reg> Matchmaker<'reg> {

    pub fn new(max_tier: u8,) -> Self {
        Self {
            max_tier,
            queue_tiers: HashMap::new(),
            queue_tanks: Vec::new(),
            last_wait_duration: Duration::ZERO,
        }
    }

    /// Number of tanks in the queue.
    pub fn len(&self) -> usize {
        self.queue_tanks.len()
    }

    /// Queue a new tank in the match maker.
    pub fn queue(&mut self, tank: &'reg Tank) {

        if tank.tier() > self.max_tier {
            panic!("above max tier");
        }

        // Calculate the sub class that is specialize the tank class.
        let sub_class = match tank.name() {
            "EBR 105" |
            "EBR 90" |
            "Lynx 6x6" |
            "EBR Hotch." => TankSubClass::Wheeled,
            "Bourrasque" |
            "Miel" => TankSubClass::Bourrasque,
            _ => TankSubClass::Standard,
        };

        // Calculate the tier range of this tank.
        let tier = tank.tier() as i16;
        let (min_tier, max_tier) = match tank.class() {
            _ if tier == 1          => (tier, tier + 1),
            TankClass::Light        => (tier - 2, tier + 3),
            TankClass::Medium       => (tier - 2, tier + 2),
            TankClass::Heavy        => (tier - 2, tier + 2),
            TankClass::Destroyer    => (tier - 2, tier + 2),
            TankClass::Clicker      => (tier - 1, tier + 3),
        };
        let min_tier = min_tier.max(1) as u8;
        let max_tier = max_tier.min(self.max_tier as i16) as u8;

        // Update or register the queue tier.
        let now: Instant = Instant::now();
        match self.queue_tiers.entry(tank.tier()) {
            hash_map::Entry::Occupied(mut o) => {
                o.get_mut().count += 1;
                o.get_mut().last_queue_time = now;
            }
            hash_map::Entry::Vacant(v) => {
                v.insert(QueueTier {
                    count: 1,
                    last_queue_time: now,
                    last_match_time: None,
                });
            }
        }

        // Finally queue the tank!
        self.queue_tanks.push(QueueTank {
            tank,
            queue_time: now,
            sub_class,
            min_tier,
            max_tier,
        });

    }

    pub fn poll(&mut self, teams_count: usize, team_size: usize) -> Option<Match<'reg>> {

        // println!("tanks: {}", self.queue_tanks.len());
        // print!("tiers: ");
        // for tier in 1..=self.max_tier {
        //     print!("{tier:2}:{:<3} ", self.queue_tiers.get(&tier).map(|queue_tier| queue_tier.count).unwrap_or_default());
        // }
        // println!();

        assert!(teams_count >= 2, "too few teams");
        assert!(teams_count <= 32, "too much teams");
        assert!(team_size >= 1, "team size too small");

        let teams_mask = u32::MAX >> (32 - teams_count);
        let mut rng = rand::rng();

        // Compute the total number of tanks expected.
        let tanks_count = teams_count * team_size;
        if self.queue_tanks.len() < tanks_count {
            return None;
        }

        // This is the oldest tank in the queue, if its wait time is too high, we favorise
        // matchmaking its tier.
        let longest_wait_tank = &self.queue_tanks[0];
        let longest_wait_duration = longest_wait_tank.queue_time.elapsed();
        let longest_wait_abnormal = !self.last_wait_duration.is_zero() && 
            (longest_wait_duration.as_secs_f32() / self.last_wait_duration.as_secs_f32()) >= 10.0;

        let handle_longest_wait = longest_wait_abnormal && rng.random_ratio(1, 10);

        // Pick the base tier.
        let (tier, queue_tier) = if handle_longest_wait {
            let tier = longest_wait_tank.tank.tier();
            (tier, self.queue_tiers.get(&tier).unwrap())
        } else {
            let mut x = rng.random_range(0..self.queue_tanks.len());
            self.queue_tiers.iter().find_map(|(&tier, queue_tier)| {
                if x < queue_tier.count {
                    Some((tier, queue_tier))
                } else {
                    x -= queue_tier.count;
                    None
                }
            }).unwrap()
        };

        // Compute the ratio of this tier between all other tiers. We expect every tier
        // to be equally distributed, so every tier should be around '1 / tiers_count'.
        let tier_ratio = queue_tier.count as f32 / self.queue_tanks.len() as f32;
        let tier_ratio_drift = tier_ratio / (1.0 / self.queue_tiers.len() as f32);

        // We change the tier spread with the drift, every factor of 3 we reduce one, 
        // naturally saturating at 0, so no spread if the tier drift with a factor of 6.
        let tier_spread = if handle_longest_wait {
            2
        } else {
            2u32.saturating_sub(tier_ratio_drift as u32 / 3) as u8
        };

        // Random tier distribution.
        let min_tier = (tier as i16 - rng.random_range(0..=tier_spread) as i16).max(1) as u8;
        let max_tier = (tier + rng.random_range(0..=tier_spread)).min(self.max_tier);

        // Each constraint for each slot in a team, all teams have the same constraints,
        // the initial constraint only contains the required tier, this cannot be changed,
        // when a tank is first added at a role it will define more constraints.
        let mut slots_constraints = Vec::new();
        for tier in min_tier..=max_tier {

            let missing_count = team_size - slots_constraints.len();
            let available_count = match self.queue_tiers.get(&tier) {
                Some(queue_tier) => queue_tier.count / teams_count,
                None => 0,
            };

            let slots_count = 
            if tier == max_tier {

                if handle_longest_wait {
                    // If we need to handle longest wait, we ignore the missing count 
                    // and we create teams with less tanks than the requested team size.
                    available_count.min(missing_count)
                } else if available_count < missing_count {
                    // We need to fill the remaining slots of the team, so we use the missing
                    // count and abort if the available count at this tier is not enough.
                    return None;
                } else {
                    missing_count
                }

                // Technically it breaks here...
                // break;

            } else {

                let missing_min_count = missing_count / 2;
                if missing_min_count == 0 {
                    continue;  // There is 0 or 1 remaining slot, keep it for the max tier.
                }

                let missing_max_count = (missing_min_count + 1).min(missing_count);
                rng.random_range(missing_min_count..=missing_max_count).min(available_count)

            };

            for _ in 0..slots_count {
                slots_constraints.push(TeamSlotConstraints {
                    free_teams: teams_mask,
                    tier,
                    class: None,
                    sub_class: None,
                });
            }

        }

        // Recompute the actual teams size and expected tanks count.
        let team_size = slots_constraints.len();
        let tanks_count = teams_count * team_size;

        if team_size < 7 {
            return None;
        }

        // Now find the tanks by their indices.
        let mut slots = Vec::with_capacity(tanks_count);
        for (tank_index, tank) in self.queue_tanks.iter().enumerate() {
            
            // Exclude tanks that are not in the good range.
            if tank.min_tier > min_tier || tank.max_tier < max_tier {
                continue;
            }

            // Mutate because if we find a slot, we also find a team.
            for constraint in &mut slots_constraints {
                
                // Ignore this slot if all teams have found a player for it.
                if constraint.free_teams == 0 {
                    continue;
                }

                // Ignore this slot if tier isn't matching.
                if constraint.tier != tank.tank.tier() {
                    continue;
                }

                // Ignore this slot if class isn't matching, or set the class.
                if let Some(class) = constraint.class {
                    if !handle_longest_wait && class != tank.tank.class() {
                        continue;
                    }
                } else {
                    constraint.class = Some(tank.tank.class());
                }

                // Ignore this slot if class isn't matching, or set the class.
                if let Some(sub_class) = constraint.sub_class {
                    if !handle_longest_wait && sub_class != tank.sub_class {
                        continue;
                    }
                } else {
                    constraint.sub_class = Some(tank.sub_class);
                }

                // We found a valid slot, we choose the team!
                let mut one_index = rng.random_range(0..constraint.free_teams.count_ones());
                // We transform the one's index to the actual team index.
                for team_index in 0..teams_count {
                    if constraint.free_teams & (1 << team_index) != 0 {
                        if one_index == 0 {
                            // We zero-out this bit, we
                            constraint.free_teams &= !(1 << team_index);
                            slots.push(TeamSlot {
                                team_index,
                                tank_index,
                            });
                            break;
                        } else {
                            one_index -= 1;
                        }
                    }
                }

                break;

            }

            if slots.len() == tanks_count {
                break;
            }

        }

        if slots.len() != tanks_count {
            return None;
        }

        // Get a new instant after the matchmaking algorithm.
        let now = Instant::now();
        let mut tanks = Vec::with_capacity(tanks_count);

        // We go in reverse order because tank index will be decreasing, and therefore
        // removing them from the queue will not shuffle the remaining indices.
        self.last_wait_duration = Duration::ZERO;
        for slot in slots.into_iter().rev() {

            // Definitely un-queue the tank.
            let queue_tank = self.queue_tanks.remove(slot.tank_index);

            // Decrement its tier count.
            let queue_tier = self.queue_tiers.get_mut(&queue_tank.tank.tier()).unwrap();
            queue_tier.last_match_time = Some(now);
            queue_tier.count -= 1;

            // Compute wait duration, and recompute the last average wait time.
            let wait_duration = now - queue_tank.queue_time;
            self.last_wait_duration += wait_duration;

            tanks.push(MatchTank {
                tank: queue_tank.tank,
                team_index: slot.team_index,
                wait_duration,
            });

        }

        // Compute the average!
        self.last_wait_duration /= tanks.len() as u32;

        // Sort to ensure that tanks are in their respective team, and we also sort them
        // by tier, class and lastly by their name.
        tanks.sort_by(|a, b| {
            Ord::cmp(&a.team_index, &b.team_index)
                .then(Ord::cmp(&b.tank.tier(), &a.tank.tier()))
                .then(Ord::cmp(&a.tank.class(), &b.tank.class()))
                .then(Ord::cmp(a.tank.name(), b.tank.name()))
        });

        Some(Match {
            tanks,
            team_size,
            wait_duration: self.last_wait_duration,
        })

    }

}

/// A completed match.
#[derive(Debug)]
pub struct Match<'reg> {
    tanks: Vec<MatchTank<'reg>>,
    team_size: usize,
    wait_duration: Duration,
}

impl<'reg> Match<'reg> {

    pub fn team_size(&self) -> usize {
        self.team_size
    }

    pub fn teams_count(&self) -> usize {
        self.tanks.len() / self.team_size
    }

    pub fn tanks_count(&self) -> usize {
        self.tanks.len()
    }

    pub fn team_tanks(&self, index: usize) -> &[MatchTank<'reg>] {
        let start = self.team_size * index;
        assert!(start < self.tanks.len(), "invalid team index");
        &self.tanks[start..][..self.team_size]
    }

    pub fn tank(&self, team_index: usize, tank_index: usize) -> &MatchTank<'reg> {
        assert!(tank_index < self.team_size, "invalid tank index");
        let index = team_index * self.team_size + tank_index;
        assert!(index < self.tanks.len(), "invalid team index");
        &self.tanks[index]
    }
    
    pub fn wait_duration(&self) -> Duration {
        self.wait_duration
    }

}

/// A tank in the match.
#[derive(Debug)]
pub struct MatchTank<'reg> {
    tank: &'reg Tank,
    team_index: usize,
    wait_duration: Duration,
}

impl<'reg> MatchTank<'reg> {

    pub fn tank(&self) -> &'reg Tank {
        self.tank
    }

    pub fn team_index(&self) -> usize {
        self.team_index
    }

    pub fn wait_duration(&self) -> Duration {
        self.wait_duration
    }

}

/// A tank role, which is more precise and can be different from the tank class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TankSubClass {
    Standard,
    Wheeled,
    Bourrasque,
}

/// Internal structure to keep track of a tank that has been queued in the matchmaker, 
/// this is 
#[derive(Debug)]
struct QueueTank<'reg> {
    tank: &'reg Tank,
    queue_time: Instant,
    sub_class: TankSubClass,
    min_tier: u8,
    max_tier: u8,
}

/// Internal structure to keep track of a tier and the number of tanks waiting in it.
#[derive(Debug)]
struct QueueTier {
    count: usize,
    last_queue_time: Instant,
    last_match_time: Option<Instant>,
}

/// An equal constraint for all teams when searching tanks that fits.
#[derive(Debug)]
struct TeamSlotConstraints {
    /// Bit vector that defines in which teams this slot is still free.
    free_teams: u32,
    /// The tier constraint for that slot.
    tier: u8,
    /// The class constraint for that slot.
    class: Option<TankClass>,
    /// The subclass constraint for that slot.
    sub_class: Option<TankSubClass>,
}

#[derive(Debug)]
struct TeamSlot {
    team_index: usize,
    tank_index: usize,
}
