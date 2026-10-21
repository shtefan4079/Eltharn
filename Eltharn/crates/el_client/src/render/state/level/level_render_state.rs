use super::camera_render_state::CameraRenderState;
use super::sky_render_state::SkyRenderState;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRenderState {
    pub camera_render_state: CameraRenderState,
    pub sky_render_state: SkyRenderState,
    pub game_time: i64,
    pub day_time: i64,
    pub partial_tick: f32,
    pub cloud_color: [f32; 4],
    pub cloud_height: f32,
    pub view_distance_blocks: f32,
    pub is_in_world: bool,
}
