//! The tank registry.

use std::sync::Arc;

use rand::Rng;


/// Registry of all tanks and their pick rate.
#[derive(Debug)]
pub struct TankRegistry {
    tanks: Vec<Tank>,
    battles: u32,
}

impl TankRegistry {

    pub fn new() -> Self {
        Self {
            tanks: Vec::new(),
            battles: 0,
        }
    }

    /// Length of this registry, the number of registered tanks.
    pub fn len(&self) -> usize {
        self.tanks.len()
    }

    /// The 
    pub fn battles(&self) -> u32 {
        self.battles
    }

    pub fn register(&mut self, name: &str, tier: u8, class: TankClass, battles: u32) {

        let name = Arc::<str>::from(name);

        self.battles += battles;
        self.tanks.push(Tank {
            name,
            tier,
            class,
            battles,
        });

    }

    /// Return an iterator that indefinitely return a random rank from the registry, 
    /// weighted from its battle pick rate as registered.
    pub fn pick_many_random(&self) -> impl Iterator<Item = &'_ Tank> + use<'_> {
        let mut rng = rand::rng();
        std::iter::repeat_with(move || {
            let mut x = rng.random_range(..self.battles);
            self.tanks.iter().find(|tank| {
                if x < tank.battles {
                    true
                } else {
                    x -= tank.battles;
                    false
                }
            }).unwrap()
        })
    }

    /// Pick a single random tank, see [`Self::pick_many_random`].
    pub fn pick_random(&self) -> &Tank {
        self.pick_many_random().next().unwrap()
    }

}

/// Represent a tank.
#[derive(Debug)]
pub struct Tank {
    name: Arc<str>,
    tier: u8,
    class: TankClass,
    battles: u32,
}

impl Tank {

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tier(&self) -> u8 {
        self.tier
    }

    pub fn class(&self) -> TankClass {
        self.class
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TankClass {
    Heavy,
    Medium,
    Destroyer,
    Light,
    Clicker,
}
