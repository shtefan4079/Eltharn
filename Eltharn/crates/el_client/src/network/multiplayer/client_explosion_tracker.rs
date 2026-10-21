#![allow(dead_code)]

use glam::Vec3;

#[derive(Debug, Clone)]
pub struct ExplosionInfo {
    pub center: Vec3,
    pub radius: f32,
    pub block_count: i32,
}

pub struct ClientExplosionTracker {
    pub explosions: Vec<ExplosionInfo>,
}

impl Default for ClientExplosionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientExplosionTracker {
    pub fn new() -> Self {
        Self {
            explosions: Vec::new(),
        }
    }

    pub fn track(&mut self, center: Vec3, radius: f32, block_count: i32) {
        self.explosions.push(ExplosionInfo {
            center,
            radius,
            block_count,
        });
    }

    pub fn tick(&mut self) {
        self.explosions.clear();
    }
}
