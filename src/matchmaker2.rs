use std::collections::HashMap;

use crate::tank::{Tank, TankClass};


/// Second iteration of the matchmaker algorithm.
#[derive(Debug)]
pub struct Matchmaker<'reg> {
    matches: Vec<QueueMatch<'reg>>,
}

impl<'reg> Matchmaker<'reg> {

    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
        }
    }
    
    /// Queue a tank in the matchmaker.
    pub fn queue(&mut self, player: Player<'reg>) -> Option<Match<'reg>> {

        // println!("tank: {tank:?}");

        // Calculate the tier range of this tank.
        let tier = player.tank.tier();
        let class = player.tank.class();

        let (min_tier, max_tier) = match (tier, class) {
            (1 | 2 | 3, _)                  => (1, 1),
            (_, TankClass::Heavy)           => (2, 2),
            (_, TankClass::Medium)          => (2, 2),
            (_, TankClass::Light)           => (2, 3),
            (_, TankClass::Destroyer)       => (2, 2),
            (_, TankClass::Clicker)         => (1, 3), 
        };

        let min_tier = (tier as i16 - min_tier).max(1) as u8;
        let max_tier = (tier as i16 + max_tier).min(10) as u8;

        let mut candidates = Vec::new();

        // Try to find a match where this player fits.
        for (match_index, queue_match) in self.matches.iter().enumerate() {
            
            // First, pick a team that has the right tier range.
            if min_tier > queue_match.limit_min_tier 
            || max_tier < queue_match.limit_max_tier {

                // Secondary condition to allow a tank to be included with a stricter
                // tier range, only if no tanks in the team are outside of this range.
                if queue_match.actual_min_tier < min_tier
                || queue_match.actual_max_tier > max_tier {
                    continue;
                }

            }

            // Match tier range is used to prefer games that are of the same tier.
            let match_tier_range = queue_match.actual_max_tier - queue_match.actual_min_tier;

            // Tier range diff from extremums.
            let tier_range_diff = if tier < queue_match.actual_min_tier {
                queue_match.actual_min_tier - tier
            } else if tier > queue_match.actual_max_tier {
                tier - queue_match.actual_max_tier
            } else {
                0
            };

            // Find info about tier and the class in that match.
            let match_tier = queue_match.tiers.get(&tier);
            let match_tier_count = match_tier
                .map(|info| info.count)
                .unwrap_or(0);
            let match_class_count = match_tier
                .and_then(|info| info.classes.get(&class).copied())
                .unwrap_or(0);

            // Compute how many tanks of each type should be in each team.
            let match_tier_balance = match_tier_count.div_ceil(queue_match.teams.len());
            let match_class_balance = match_class_count.div_ceil(queue_match.teams.len());
            
            // Here we check every team to find the good one.
            for (team_index, queue_team) in queue_match.teams.iter().enumerate() {
                
                // Ignore teams that are already full!
                if queue_team.players.len() >= queue_match.max_teams_size {
                    continue;
                }

                // Find info about tier and class in that team, to compare with those of
                // the team and favorise more balanced distributions.
                let team_tier = queue_team.tiers.get(&tier);
                let team_tier_count = team_tier
                    .map(|info| info.count)
                    .unwrap_or(0);
                let team_class_count = team_tier
                    .and_then(|info| info.classes.get(&class).copied())
                    .unwrap_or(0);

                // Check each team, if it is near or far from expected balance, it will
                // be below 0.0 if the team already has too much tanks of that type, and
                // above 0.0 if there are missing missing tanks.
                let team_tier_diff = match_tier_balance as isize - team_tier_count as isize;
                let team_class_diff = match_class_balance as isize - team_class_count as isize;
                
                // Ignore this team if it already has two more tanks of the same type,
                // which is huge.
                if team_tier_diff <= -1 || team_class_diff <= -1 {
                    continue;
                }

                // We try to build a score that will be below zero for bad candidates.
                let score = 
                    (1.0 - (match_tier_range as f32 / 3.0).max(0.0)) +
                    (1.0 - (tier_range_diff as f32 / 2.0).max(0.0)) +
                    (1.0 + (team_tier_diff as f32).clamp(-1.0, 1.0)) +
                    (1.0 + (team_class_diff as f32).clamp(-1.0, 1.0));
                
                candidates.push(QueueCandidate {
                    match_index,
                    team_index,
                    score,
                });

            }

        }

        // If no candidate, create a new game for that player, or else sort all.
        if candidates.is_empty() {

            candidates.push(QueueCandidate {
                match_index: self.matches.len(),
                team_index: 0,
                score: 0.0,
            });

            self.matches.push(QueueMatch {
                teams: (0..2).map(|_| QueueTeam {
                    players: Vec::new(),
                    tiers: HashMap::new(),
                }).collect::<Vec<_>>().into_boxed_slice(),
                max_teams_size: 15,
                limit_min_tier: min_tier,
                limit_max_tier: max_tier,
                actual_min_tier: tier,
                actual_max_tier: tier,
                tiers: HashMap::new(),
                players_count: 0,
            });

        } else {
            candidates.sort_by(|a, b| f32::total_cmp(&b.score, &a.score));
        }

        // Now we must have at least one candidate
        let best_candidate = candidates.into_iter().next().unwrap();
        let best_match = &mut self.matches[best_candidate.match_index];
        
        // We know that we can force update the allowed tier range.
        best_match.limit_min_tier = min_tier;
        best_match.limit_max_tier = max_tier;

        // Update actual actual tier range.
        let mut update_tiers = false;
        if tier < best_match.actual_min_tier {
            best_match.actual_min_tier = tier;
            update_tiers = true;
        }
        if tier > best_match.actual_max_tier {
            best_match.actual_max_tier = tier;
            update_tiers = true;
        }

        // If actual min or max tiers have been modified, ensure that they exists in the
        // tiers info map of the match and all teams.
        if update_tiers {

            let mut remaining = best_match.max_teams_size;
            for tier in best_match.actual_min_tier..=best_match.actual_max_tier {
                
                let limit = remaining / 2;
                remaining -= limit;

                for team in &mut best_match.teams {
                    let team_tier = team.tiers.entry(tier).or_default();
                    team_tier.limit = limit;
                }

                let match_tier = best_match.tiers.entry(tier).or_default();
                match_tier.limit = limit * best_match.teams.len();

            }

        }

        let best_team = &mut best_match.teams[best_candidate.team_index];

        // Update tier informations in match and team.
        let match_tier = best_match.tiers.entry(tier).or_default();
        match_tier.count += 1;
        *match_tier.classes.entry(class).or_default() += 1;
        let team_tier = best_team.tiers.entry(tier).or_default();
        team_tier.count += 1;
        *team_tier.classes.entry(class).or_default() += 1;

        // Ideally, we want tier repartition in each team to be approx (+ 0 or 1 player) 
        // half of the remaining slots, starting from the lowest tier. For example
        // in a tier 8,9,10 match, tier 8 have 15/2=7, so in range 7..8, say we take 8,
        // tier 9 have (15-8)/2=3, so in range 3..=4, say we take 3, and tier 10 has the
        // remaining slots (15-8-3)=4.
        
        
        // Finally add the player.
        best_match.players_count += 1;
        best_team.players.push(QueuePlayer {
            inner: player,
            min_tier,
            max_tier,
        });

        // The match is full!
        let target_teams_size = best_match.max_teams_size;
        if best_match.players_count == target_teams_size * best_match.teams.len() {

            let complete_match = self.matches.swap_remove(best_candidate.match_index);

            let mut ret_match = Match {
                players: Vec::new(),
                teams_size: target_teams_size,
            };

            for mut team in complete_match.teams {

                team.players.sort_by(|a, b| {
                    Ord::cmp(&b.inner.tank.tier(), &a.inner.tank.tier())
                        .then(Ord::cmp(&a.inner.tank.class(), &b.inner.tank.class()))
                        .then(Ord::cmp(a.inner.tank.name(), b.inner.tank.name()))
                });

                for player in team.players {
                    ret_match.players.push(player.inner);
                }

            }

            Some(ret_match)

        } else {
            None
        }

    }

}

/// A match queued with players into, ready to play.
#[derive(Debug)]
struct QueueMatch<'reg> {
    /// All teams in this match, with a defined size at construction depending on the
    /// expected team size.
    teams: Box<[QueueTeam<'reg>]>,
    /// The maximum size in tanks for each team in a match.
    max_teams_size: usize,
    /// The minimum tier for tanks that can play this match.
    limit_min_tier: u8,
    /// The maximum tier for tanks that can play this match.
    limit_max_tier: u8,
    /// The current minimum tier between all tanks in all teams.
    actual_min_tier: u8,
    /// The current minimum tier between all tanks in all teams.
    actual_max_tier: u8,
    /// Contains the number of tanks at that tier in all teams.
    tiers: HashMap<u8, QueueTierInfo>,
    /// Total count of players in all teams.
    players_count: usize,
}

/// A team queued into a match.
#[derive(Debug)]
struct QueueTeam<'reg> {
    /// All tanks in the team, filled when tanks are queued in the team.
    players: Vec<QueuePlayer<'reg>>,
    /// Contains the number of tanks at that tier in the team.
    tiers: HashMap<u8, QueueTierInfo>,
}

/// Information about a tier in a match or a team to ease matchmaking.
#[derive(Debug, Default)]
struct QueueTierInfo {
    /// The number of players of that tier.
    count: usize,
    /// The expected maximum count of this tier, recomputed each time the actual tier
    /// range is modified. This soft limit can be crossed but with penalty, when crossing
    /// this limit other tiers should be updated accordingly to ensure that the sum of
    /// all soft limits is the number of total players.
    limit: usize,

    classes: HashMap<TankClass, usize>,
}

/// A tank that has been queued.
#[derive(Debug)]
struct QueuePlayer<'reg> {
    /// The actual tank definition from the registry.
    inner: Player<'reg>,
    /// The minimum tier this tank can play with in the same game.
    min_tier: u8,
    /// The maximum tier this tank can play with in the same game.
    max_tier: u8,
}

/// When queueing a tank, this represent one match that is candidate for the player to
/// be placed into. There are many metrics that allows sorting out the best candidate.
#[derive(Debug)]
struct QueueCandidate {
    match_index: usize,
    team_index: usize,
    score: f32,
}

/// Represent a player and its configuration.
#[derive(Debug)]
pub struct Player<'reg> {
    tank: &'reg Tank,
}

impl<'reg> Player<'reg> {

    pub fn new(tank: &'reg Tank) -> Self {
        Self {
            tank,
        }
    }

    #[inline]
    pub fn tank(&self) -> &'reg Tank {
        self.tank
    }

}

/// A completed match.
#[derive(Debug)]
pub struct Match<'reg> {
    players: Vec<Player<'reg>>,
    teams_size: usize,
}

impl<'reg> Match<'reg> {

    #[inline]
    pub fn teams_size(&self) -> usize {
        self.teams_size
    }

    #[inline]
    pub fn teams_count(&self) -> usize {
        self.players.len() / self.teams_size
    }

    #[inline]
    pub fn players_count(&self) -> usize {
        self.players.len()
    }

    pub fn team_players(&self, team_index: usize) -> &[Player<'reg>] {
        let start = self.teams_size * team_index;
        assert!(start < self.players.len(), "invalid team index");
        &self.players[start..][..self.teams_size]
    }

    pub fn player(&self, team_index: usize, tank_index: usize) -> &Player<'reg> {
        assert!(tank_index < self.teams_size, "invalid tank index");
        let index = team_index * self.teams_size + tank_index;
        assert!(index < self.players.len(), "invalid team index");
        &self.players[index]
    }

}
