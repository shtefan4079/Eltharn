use crate::gameplay::player::LocalPlayer;
use el_mod_content::scene_types::player::PlayerContext;
pub(crate) fn sample(
    player: &LocalPlayer,
    dimension: Option<&str>,
    world: u64,
    epoch: u64,
) -> Result<PlayerContext, String> {
    let context = PlayerContext {
        version: 1,
        world: world.to_string(),
        epoch: epoch.to_string(),
        dimension: dimension.map(str::to_owned),
        position: player.exact_position().to_array(),
        yaw_degrees: f64::from(player.y_rot_degrees()),
        pitch_degrees: f64::from(player.x_rot_degrees()),
    };
    context.validate()?;
    Ok(context)
}
