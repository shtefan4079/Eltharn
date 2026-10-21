use crate::gameplay::player::LocalPlayer;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseTurn {
    pub yaw_delta_degrees: f32,
    pub pitch_delta_degrees: f32,
    pub yaw_delta: f32,
    pub pitch_delta: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseTurnOptions {
    pub sensitivity: f64,
    pub invert_x: bool,
    pub invert_y: bool,
    pub smooth_camera: bool,
    pub first_person: bool,
    pub scoping: bool,
}
impl Default for MouseTurnOptions {
    fn default() -> Self {
        Self {
            sensitivity: 0.5,
            invert_x: false,
            invert_y: false,
            smooth_camera: false,
            first_person: true,
            scoping: false,
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SmoothDouble {
    target_value: f64,
    remaining_value: f64,
    last_amount: f64,
}
impl SmoothDouble {
    pub fn new_delta_value(&mut self, target_delta: f64, time: f64) -> f64 {
        self.target_value += target_delta;
        let mut delta = self.target_value - self.remaining_value;
        let new_last_amount = self.last_amount + 0.5 * (delta - self.last_amount);
        let delta_sign = delta.signum();
        if delta_sign * delta > delta_sign * self.last_amount {
            delta = new_last_amount;
        }
        self.last_amount = new_last_amount;
        self.remaining_value += delta * time;
        delta * time
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct MouseHandler {
    left_pressed: bool,
    middle_pressed: bool,
    right_pressed: bool,
    xpos: f64,
    ypos: f64,
    ignore_first_move: bool,
    accumulated_dx: f64,
    accumulated_dy: f64,
    smooth_turn_x: SmoothDouble,
    smooth_turn_y: SmoothDouble,
    mouse_grabbed: bool,
}
impl Default for MouseHandler {
    fn default() -> Self {
        Self {
            left_pressed: false,
            middle_pressed: false,
            right_pressed: false,
            xpos: 0.0,
            ypos: 0.0,
            ignore_first_move: true,
            accumulated_dx: 0.0,
            accumulated_dy: 0.0,
            smooth_turn_x: SmoothDouble::default(),
            smooth_turn_y: SmoothDouble::default(),
            mouse_grabbed: false,
        }
    }
}
#[allow(dead_code)]
impl MouseHandler {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn calculate_turn(
        sensitivity: f64,
        dx: f64,
        dy: f64,
        invert_x: bool,
        invert_y: bool,
    ) -> MouseTurn {
        let (_, sensitivity_scale) = java_mouse_sensitivity(sensitivity);
        let yaw_delta_degrees = (dx * sensitivity_scale) as f32 * 0.15_f32;
        let pitch_delta_degrees = (dy * sensitivity_scale) as f32 * 0.15_f32;
        let yaw_delta_degrees = if invert_x {
            -yaw_delta_degrees
        } else {
            yaw_delta_degrees
        };
        let pitch_delta_degrees = if invert_y {
            -pitch_delta_degrees
        } else {
            pitch_delta_degrees
        };
        MouseTurn {
            yaw_delta_degrees,
            pitch_delta_degrees,
            yaw_delta: yaw_delta_degrees.to_radians(),
            pitch_delta: pitch_delta_degrees.to_radians(),
        }
    }
    pub fn turn_player(player: &mut LocalPlayer, turn: MouseTurn) {
        player.turn_view_degrees(turn.yaw_delta_degrees, turn.pitch_delta_degrees);
    }
    pub fn on_button(&mut self, button: u16, pressed: bool) {
        match button {
            0 => self.left_pressed = pressed,
            1 => self.right_pressed = pressed,
            2 => self.middle_pressed = pressed,
            _ => {}
        }
    }
    pub fn on_move(&mut self, xpos: f64, ypos: f64, window_active: bool) {
        if self.ignore_first_move {
            self.xpos = xpos;
            self.ypos = ypos;
            self.ignore_first_move = false;
            return;
        }
        if window_active {
            self.accumulated_dx += xpos - self.xpos;
            self.accumulated_dy += ypos - self.ypos;
        }
        self.xpos = xpos;
        self.ypos = ypos;
    }
    pub fn accumulate_relative_motion(&mut self, dx: f64, dy: f64, window_active: bool) {
        if window_active {
            self.accumulated_dx += dx;
            self.accumulated_dy += dy;
        }
    }
    pub fn discard_accumulated_motion(&mut self) {
        self.accumulated_dx = 0.0;
        self.accumulated_dy = 0.0;
    }
    pub fn handle_accumulated_movement(
        &mut self,
        elapsed_seconds: f64,
        options: MouseTurnOptions,
    ) -> Option<MouseTurn> {
        let dx = self.accumulated_dx;
        let dy = self.accumulated_dy;
        self.accumulated_dx = 0.0;
        self.accumulated_dy = 0.0;
        if !self.mouse_grabbed || (dx == 0.0 && dy == 0.0) {
            return None;
        }
        let (sensitivity_mod, sensitivity) = java_mouse_sensitivity(options.sensitivity);
        let (xo, yo) = if options.smooth_camera {
            (
                self.smooth_turn_x
                    .new_delta_value(dx * sensitivity, elapsed_seconds * sensitivity),
                self.smooth_turn_y
                    .new_delta_value(dy * sensitivity, elapsed_seconds * sensitivity),
            )
        } else if options.first_person && options.scoping {
            self.smooth_turn_x.reset();
            self.smooth_turn_y.reset();
            (dx * sensitivity_mod, dy * sensitivity_mod)
        } else {
            self.smooth_turn_x.reset();
            self.smooth_turn_y.reset();
            (dx * sensitivity, dy * sensitivity)
        };
        let yaw_delta_degrees = xo as f32 * 0.15;
        let pitch_delta_degrees = yo as f32 * 0.15;
        let yaw_delta_degrees = if options.invert_x {
            -yaw_delta_degrees
        } else {
            yaw_delta_degrees
        };
        let pitch_delta_degrees = if options.invert_y {
            -pitch_delta_degrees
        } else {
            pitch_delta_degrees
        };
        Some(MouseTurn {
            yaw_delta_degrees,
            pitch_delta_degrees,
            yaw_delta: yaw_delta_degrees.to_radians(),
            pitch_delta: pitch_delta_degrees.to_radians(),
        })
    }
    pub const fn is_left_pressed(&self) -> bool {
        self.left_pressed
    }
    pub const fn is_middle_pressed(&self) -> bool {
        self.middle_pressed
    }
    pub const fn is_right_pressed(&self) -> bool {
        self.right_pressed
    }
    pub const fn xpos(&self) -> f64 {
        self.xpos
    }
    pub const fn ypos(&self) -> f64 {
        self.ypos
    }
    pub fn set_ignore_first_move(&mut self) {
        self.ignore_first_move = true;
    }
    pub const fn is_mouse_grabbed(&self) -> bool {
        self.mouse_grabbed
    }
    pub fn set_mouse_grabbed(&mut self, mouse_grabbed: bool) {
        if self.mouse_grabbed == mouse_grabbed {
            return;
        }
        self.mouse_grabbed = mouse_grabbed;
        self.accumulated_dx = 0.0;
        self.accumulated_dy = 0.0;
        if !mouse_grabbed {
            self.smooth_turn_x.reset();
            self.smooth_turn_y.reset();
        }
    }
    pub fn update_mouse_grabbed(
        &mut self,
        mouse_grabbed: bool,
        screen_width: u32,
        screen_height: u32,
    ) -> Option<(f64, f64)> {
        if self.mouse_grabbed == mouse_grabbed {
            return None;
        }
        if mouse_grabbed {
            self.grab_mouse(f64::from(screen_width), f64::from(screen_height));
        } else {
            self.release_mouse(f64::from(screen_width), f64::from(screen_height));
        }
        Some((self.xpos, self.ypos))
    }
    pub fn grab_mouse(&mut self, screen_width: f64, screen_height: f64) {
        if !self.mouse_grabbed {
            self.set_mouse_grabbed(true);
            self.xpos = (screen_width / 2.0).floor();
            self.ypos = (screen_height / 2.0).floor();
            self.ignore_first_move = true;
        }
    }
    pub fn release_mouse(&mut self, screen_width: f64, screen_height: f64) {
        if self.mouse_grabbed {
            self.set_mouse_grabbed(false);
            self.xpos = (screen_width / 2.0).floor();
            self.ypos = (screen_height / 2.0).floor();
        }
    }
    pub fn cursor_entered(&mut self) {
        self.ignore_first_move = true;
    }
    pub fn scaled_x_pos(x: f64, gui_width: f64, screen_width: f64) -> f64 {
        x * gui_width / screen_width
    }
    pub fn scaled_y_pos(y: f64, gui_height: f64, screen_height: f64) -> f64 {
        y * gui_height / screen_height
    }
}
fn java_mouse_sensitivity(sensitivity: f64) -> (f64, f64) {
    let ss = sensitivity * f64::from(0.6_f32) + f64::from(0.2_f32);
    let sensitivity_mod = ss * ss * ss;
    (sensitivity_mod, sensitivity_mod * 8.0)
}
