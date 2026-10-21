use crate::gameplay::camera::Camera;
use crate::gameplay::camera_type::CameraType;
use crate::render::state::level::{CameraRenderState, LevelRenderState};
use crate::world::client_level::ClientLevel;
pub const fn should_render_camera_entity(
    camera_type: CameraType,
    is_camera_entity: bool,
    is_sleeping: bool,
) -> bool {
    !is_camera_entity || camera_type.is_detached() || is_sleeping
}
pub const fn should_render_local_player(camera_type: CameraType, is_sleeping: bool) -> bool {
    should_render_camera_entity(camera_type, true, is_sleeping)
}
pub fn extract_level_render_state_with_overrides(
    camera: &Camera,
    level: Option<&ClientLevel>,
    partial_tick: f32,
    effective_render_distance_chunks: u32,
    is_in_world: bool,
    should_render_dark_disc: bool,
    configured_moon_phase: Option<u32>,
) -> LevelRenderState {
    let camera_render_state = CameraRenderState::extract(camera);
    let game_time = level.map(|l| l.game_time()).unwrap_or(0);
    let day_time = level.map(|l| l.day_time()).unwrap_or(0);
    let rain_level = level.map(|l| l.rain_level(partial_tick)).unwrap_or(0.0);
    let thunder_level = level.map(|l| l.thunder_level(partial_tick)).unwrap_or(0.0);
    let sky_color = level
        .map(|l| l.sky_color(partial_tick))
        .unwrap_or([0.1, 0.5, 0.8]);
    let fog_color = level
        .map(|l| l.fog_color(partial_tick))
        .unwrap_or([0.75, 0.85, 1.0]);
    let view_distance_blocks = (effective_render_distance_chunks as f32 * 16.0).min(512.0);
    let moon_phase = configured_moon_phase
        .or_else(|| level.map(|lvl| lvl.moon_phase_index()))
        .unwrap_or(0);
    let timeline = crate::render::state::level::sky_render_state::SkyTimeline::new();
    let sky_render_state = timeline.extract_sky_render_state(
        day_time,
        partial_tick,
        rain_level,
        thunder_level,
        sky_color,
        fog_color,
        should_render_dark_disc,
        moon_phase,
        view_distance_blocks,
    );
    let cloud_color = level
        .map(|l| l.cloud_color_rgba(partial_tick))
        .unwrap_or([0.8, 0.8, 0.8, 0.8]);
    let cloud_height = level
        .map(|l| l.cloud_height())
        .unwrap_or(crate::world::client_level::OVERWORLD_CLOUD_HEIGHT);
    LevelRenderState {
        camera_render_state,
        sky_render_state,
        game_time,
        day_time,
        partial_tick,
        cloud_color,
        cloud_height,
        view_distance_blocks,
        is_in_world,
    }
}
