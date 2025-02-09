//! The tomato.gg tank performance format.


#[derive(Debug, serde::Deserialize)]
pub enum Nation {
    USA,
    UK,
    USSR,
    France,
    Germany,
    Italy,
    China,
    Czech,
    Poland,
    Sweden,
    Japan,
}

#[derive(Debug, serde::Deserialize)]
pub enum TankClass {
    #[serde(rename = "LT")]
    Light,
    #[serde(rename = "MT")]
    Medium,
    #[serde(rename = "HT")]
    Heavy,
    #[serde(rename = "TD")]
    Destroyer,
    #[serde(rename = "SPG")]
    Clicker,
}

#[derive(Debug, serde::Deserialize)]
pub struct TankPerf {
    pub tank_id: u32,
    pub name: String,
    pub nation: Nation,
    pub tier: u8,
    pub class: TankClass,
    pub image: String,
    pub big_image: String,
    pub battles: u32,
    pub player_wn8: f32,
    pub winrate: f32,
    pub player_winrate: f32,
    pub winrate_differential: f32,
    pub damage: u32,
    pub sniper_damage: u32,
    pub frags: f32,
    pub shots_fired: f32,
    pub direct_hits: f32,
    pub penetrations: f32,
    pub hit_rate: f32,
    pub pen_rate: f32,
    pub spotting_assist: u32,
    pub tracking_assist: u32,
    pub spots: f32,
    pub damage_blocked: u32,
    pub damage_received: u32,
    pub potential_damage_received: u32,
    pub base_capture_points: f32,
    pub base_defense_points: f32,
    pub life_time: u32,
    pub survival: f32,
    pub distance_traveled: u32,
    pub base_xp: u32,
    pub wn8: u32,
    #[serde(rename = "isPrem")]
    pub is_prem: bool,
}
