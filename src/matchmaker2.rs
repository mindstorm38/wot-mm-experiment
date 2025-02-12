use crate::tank::{Tank, TankClass};


/// Second iteration of the matchmaker algorithm.
#[derive(Debug)]
pub struct Matchmaker<'reg> {
    matches: Vec<PartialMatch<'reg>>,
}

impl<'reg> Matchmaker<'reg> {
    
    /// Queue a tank in the matchmaker.
    pub fn queue(&mut self, tank: &'reg Tank) {

        // Calculate the tier range of this tank.
        let (min, max) = match (tank.tier(), tank.class()) {
            (1 | 2 | 3, _)                  => (2, 2),
            (_, TankClass::Heavy)           => (2, 2),
            (_, TankClass::Medium)          => (2, 2),
            (_, TankClass::Light)           => (2, 3),
            (_, TankClass::Destroyer)       => (2, 2),
            (_, TankClass::Clicker)         => (1, 3), 
        };

        let min_tier = (tank.tier() as i16 + min).max(1) as u8;
        let max_tier = (tank.tier() as i16 + max).min(10) as u8;
        
        // Try to find a match where this player fits.
        for partial_match in &self.matches {
            
            // First, pick a team that has the right tier range.
            if partial_match.min_tier > min_tier 
            || partial_match.max_tier < max_tier {
                continue;
            }

            for (index, partial_team) in partial_match.teams.iter().enumerate() {
                
                // Skip this team, we just want to gather the constraints from other teams.
                if index == partial_match.next_team_index {
                    continue;
                }

            }

        }

    }

}

#[derive(Debug)]
struct PartialMatch<'reg> {
    teams: Vec<PartialTeam<'reg>>,
    min_tier: u8,
    max_tier: u8,
    next_team_index: usize,
    next_tank_index: usize,
}

#[derive(Debug)]
struct PartialTeam<'reg> {
    tanks: Vec<PartialTank<'reg>>,
}

#[derive(Debug)]
struct PartialTank<'reg> {
    tank: &'reg Tank,
    min_tier: u8,
    max_tier: u8,
}
