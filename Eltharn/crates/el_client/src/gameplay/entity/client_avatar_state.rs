use glam::Vec3;

#[derive(Debug, Clone, Default)]
pub struct ClientAvatarState {
    pub delta_movement_on_previous_tick: Vec3,
    pub walk_dist: f32,
    pub walk_dist_o: f32,
    pub x_cloak: f64,
    pub y_cloak: f64,
    pub z_cloak: f64,
    pub x_cloak_o: f64,
    pub y_cloak_o: f64,
    pub z_cloak_o: f64,
    pub bob: f32,
    pub bob_o: f32,
}

impl ClientAvatarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, position: Vec3, delta_movement: Vec3) {
        self.walk_dist_o = self.walk_dist;
        self.delta_movement_on_previous_tick = delta_movement;
        self.move_cloak(position);
    }

    pub fn add_walk_distance(&mut self, added: f32) {
        self.walk_dist += added;
    }

    pub fn delta_movement_on_previous_tick(&self) -> Vec3 {
        self.delta_movement_on_previous_tick
    }

    fn move_cloak(&mut self, pos: Vec3) {
        self.x_cloak_o = self.x_cloak;
        self.y_cloak_o = self.y_cloak;
        self.z_cloak_o = self.z_cloak;

        let x = pos.x as f64 - self.x_cloak;
        let y = pos.y as f64 - self.y_cloak;
        let z = pos.z as f64 - self.z_cloak;

        if x <= 10.0 && x >= -10.0 {
            self.x_cloak += x * 0.25;
        } else {
            self.x_cloak = pos.x as f64;
            self.x_cloak_o = self.x_cloak;
        }

        if y <= 10.0 && y >= -10.0 {
            self.y_cloak += y * 0.25;
        } else {
            self.y_cloak = pos.y as f64;
            self.y_cloak_o = self.y_cloak;
        }

        if z <= 10.0 && z >= -10.0 {
            self.z_cloak += z * 0.25;
        } else {
            self.z_cloak = pos.z as f64;
            self.z_cloak_o = self.z_cloak;
        }
    }

    pub fn get_interpolated_cloak_x(&self, partial_ticks: f32) -> f64 {
        self.x_cloak_o + (self.x_cloak - self.x_cloak_o) * partial_ticks as f64
    }

    pub fn get_interpolated_cloak_y(&self, partial_ticks: f32) -> f64 {
        self.y_cloak_o + (self.y_cloak - self.y_cloak_o) * partial_ticks as f64
    }

    pub fn get_interpolated_cloak_z(&self, partial_ticks: f32) -> f64 {
        self.z_cloak_o + (self.z_cloak - self.z_cloak_o) * partial_ticks as f64
    }

    pub fn update_bob(&mut self, t_bob: f32) {
        self.bob_o = self.bob;
        self.bob += (t_bob - self.bob) * 0.4;
    }

    pub fn reset_bob(&mut self) {
        self.bob_o = self.bob;
        self.bob = 0.0;
    }

    pub fn get_interpolated_bob(&self, partial_ticks: f32) -> f32 {
        self.bob_o + (self.bob - self.bob_o) * partial_ticks
    }

    pub fn get_backwards_interpolated_walk_distance(&self, partial_ticks: f32) -> f32 {
        let wda = self.walk_dist - self.walk_dist_o;
        -(self.walk_dist + wda * partial_ticks)
    }

    pub fn get_interpolated_walk_distance(&self, partial_ticks: f32) -> f32 {
        self.walk_dist_o + (self.walk_dist - self.walk_dist_o) * partial_ticks
    }
}
