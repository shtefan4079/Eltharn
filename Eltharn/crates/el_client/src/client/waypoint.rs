use el_protocol_backend::{BackendWaypoint, BackendWaypointData, BackendWaypointIdentifier};
use std::collections::HashMap;
pub const WAYPOINT_TRACK: u8 = 0;
pub const WAYPOINT_UNTRACK: u8 = 1;
pub const WAYPOINT_UPDATE: u8 = 2;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocatorPitchDirection {
    #[default]
    None,
    Up,
    Down,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LocatorBarMarker {
    pub style: String,
    pub color_argb: u32,
    pub angle_degrees: f32,
    pub distance: f32,
    pub pitch: LocatorPitchDirection,
}
#[derive(Clone, Debug, Default)]
pub struct ClientWaypointManager {
    waypoints: HashMap<BackendWaypointIdentifier, BackendWaypoint>,
}
impl ClientWaypointManager {
    pub fn clear(&mut self) {
        self.waypoints.clear();
    }
    pub fn has_waypoints(&self) -> bool {
        !self.waypoints.is_empty()
    }
    pub fn values(&self) -> impl Iterator<Item = &BackendWaypoint> {
        self.waypoints.values()
    }
    pub fn apply(&mut self, operation: u8, waypoint: BackendWaypoint) {
        match operation {
            WAYPOINT_TRACK => {
                self.waypoints.insert(waypoint.identifier.clone(), waypoint);
            }
            WAYPOINT_UNTRACK => {
                self.waypoints.remove(&waypoint.identifier);
            }
            WAYPOINT_UPDATE => {
                if let Some(existing) = self.waypoints.get_mut(&waypoint.identifier) {
                    if same_waypoint_data_kind(&existing.data, &waypoint.data) {
                        existing.data = waypoint.data;
                    } else {
                        log::warn!(
                            "[Waypoint] ignored Java-invalid update with a changed data type for {:?}",
                            waypoint.identifier
                        );
                    }
                }
            }
            unknown => {
                log::warn!(
                    "[Waypoint] ignored unknown tracked-waypoint operation ordinal {unknown}"
                );
            }
        }
    }
}
fn same_waypoint_data_kind(lhs: &BackendWaypointData, rhs: &BackendWaypointData) -> bool {
    std::mem::discriminant(lhs) == std::mem::discriminant(rhs)
}
pub fn java_waypoint_identifier_hash(identifier: &BackendWaypointIdentifier) -> u32 {
    match identifier {
        BackendWaypointIdentifier::Uuid(uuid) => java_uuid_hash(*uuid),
        BackendWaypointIdentifier::String(value) => java_string_hash(value),
    }
}
fn java_uuid_hash(uuid: uuid::Uuid) -> u32 {
    let bits = uuid.as_u128();
    let most = (bits >> 64) as u64;
    let least = bits as u64;
    let folded = most ^ least;
    (folded >> 32) as u32 ^ folded as u32
}
fn java_string_hash(value: &str) -> u32 {
    value.encode_utf16().fold(0u32, |hash, code_unit| {
        hash.wrapping_mul(31).wrapping_add(u32::from(code_unit))
    })
}
pub fn java_default_locator_color(identifier: &BackendWaypointIdentifier) -> u32 {
    java_argb_set_brightness(0xFF00_0000 | java_waypoint_identifier_hash(identifier), 0.9)
}
fn java_argb_set_brightness(color: u32, brightness: f32) -> u32 {
    let mut red = ((color >> 16) & 0xFF) as i32;
    let mut green = ((color >> 8) & 0xFF) as i32;
    let mut blue = (color & 0xFF) as i32;
    let alpha = ((color >> 24) & 0xFF) as i32;
    let rgb_max = red.max(green).max(blue);
    let rgb_min = red.min(green).min(blue);
    let constant_range = (rgb_max - rgb_min) as f32;
    let saturation = if rgb_max != 0 {
        constant_range / rgb_max as f32
    } else {
        0.0
    };
    let hue = if saturation == 0.0 {
        0.0
    } else {
        let constant_red = (rgb_max - red) as f32 / constant_range;
        let constant_green = (rgb_max - green) as f32 / constant_range;
        let constant_blue = (rgb_max - blue) as f32 / constant_range;
        let mut hue = if red == rgb_max {
            constant_blue - constant_green
        } else if green == rgb_max {
            2.0 + constant_red - constant_blue
        } else {
            4.0 + constant_green - constant_red
        } / 6.0;
        if hue < 0.0 {
            hue += 1.0;
        }
        hue
    };
    if saturation == 0.0 {
        let channel = java_round(brightness * 255.0);
        return java_argb(alpha, channel, channel, channel);
    }
    let segment = (hue - hue.floor()) * 6.0;
    let offset = segment - segment.floor();
    let primary = brightness * (1.0 - saturation);
    let secondary = brightness * (1.0 - saturation * offset);
    let tertiary = brightness * (1.0 - saturation * (1.0 - offset));
    match segment as i32 {
        0 => {
            red = java_round(brightness * 255.0);
            green = java_round(tertiary * 255.0);
            blue = java_round(primary * 255.0);
        }
        1 => {
            red = java_round(secondary * 255.0);
            green = java_round(brightness * 255.0);
            blue = java_round(primary * 255.0);
        }
        2 => {
            red = java_round(primary * 255.0);
            green = java_round(brightness * 255.0);
            blue = java_round(tertiary * 255.0);
        }
        3 => {
            red = java_round(primary * 255.0);
            green = java_round(secondary * 255.0);
            blue = java_round(brightness * 255.0);
        }
        4 => {
            red = java_round(tertiary * 255.0);
            green = java_round(primary * 255.0);
            blue = java_round(brightness * 255.0);
        }
        5 => {
            red = java_round(brightness * 255.0);
            green = java_round(primary * 255.0);
            blue = java_round(secondary * 255.0);
        }
        _ => unreachable!("Java ARGB hue segment is in [0, 6)"),
    }
    java_argb(alpha, red, green, blue)
}
fn java_round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}
fn java_argb(alpha: i32, red: i32, green: i32, blue: i32) -> u32 {
    ((alpha as u32 & 0xFF) << 24)
        | ((red as u32 & 0xFF) << 16)
        | ((green as u32 & 0xFF) << 8)
        | (blue as u32 & 0xFF)
}
