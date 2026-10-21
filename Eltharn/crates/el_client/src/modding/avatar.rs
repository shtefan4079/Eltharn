use crate::gameplay::player::{LocalPlayer, PlayerPose};
use el_mod_content::scene_types::avatar::{
    AvatarPose, CAPE_ROWS, CapeDeformation, CapeRows, LocalAvatarContext, LocalAvatarMotion,
    local_handle,
};
#[derive(Default)]
pub(crate) struct CapeController {
    identity: Option<(u64, u64)>,
    tick: Option<u64>,
    owner: Option<String>,
    previous: CapeRows,
    current: CapeRows,
}
impl CapeController {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(crate) fn accept<'a>(
        &mut self,
        world: u64,
        epoch: u64,
        tick: u64,
        contributions: impl IntoIterator<Item = (&'a str, &'a CapeDeformation)>,
    ) -> Result<(), String> {
        if world == 0 || epoch == 0 {
            return Err("invalid cape world/epoch".into());
        }
        if self.identity != Some((world, epoch)) {
            self.clear();
            self.identity = Some((world, epoch));
        }
        let expected = local_handle(world, epoch);
        let mut winner: Option<(&str, &CapeDeformation)> = None;
        for (owner, deformation) in contributions {
            deformation.validate()?;
            if owner.is_empty() || owner.len() > 160 || owner.contains('\0') {
                return Err("invalid cape contribution owner".into());
            }
            if deformation.handle != expected {
                return Err("stale local cape handle".into());
            }
            if winner.is_none_or(|(selected, _)| owner < selected) {
                winner = Some((owner, deformation));
            }
        }
        if self.tick.is_some_and(|last| tick < last) {
            return Err("stale local cape tick".into());
        }
        let Some((owner, deformation)) = winner else {
            self.owner = None;
            self.previous = [[0; 3]; CAPE_ROWS];
            self.current = self.previous;
            self.tick = Some(tick);
            return Ok(());
        };
        if self.tick != Some(tick) {
            self.previous = if self.owner.as_deref() == Some(owner) {
                self.current
            } else {
                [[0; 3]; CAPE_ROWS]
            };
        } else if self.owner.as_deref() != Some(owner) {
            self.previous = [[0; 3]; CAPE_ROWS];
        }
        self.owner = Some(owner.to_owned());
        self.current = deformation.offsets_pixel_milli;
        self.tick = Some(tick);
        Ok(())
    }
    pub(crate) fn rows(
        &self,
        partial_tick: f32,
    ) -> Option<crate::render::model::player::cape_deform::InterpolatedCapeRows> {
        self.owner.as_ref()?;
        Some(crate::render::model::player::cape_deform::interpolate(
            &self.previous,
            &self.current,
            partial_tick,
        ))
    }
}
pub(crate) fn sample(
    player: &LocalPlayer,
    world: u64,
    epoch: u64,
) -> Result<LocalAvatarContext, String> {
    let avatar = &player.avatar_state;
    let pose = match player.current_pose(false) {
        PlayerPose::Standing => AvatarPose::Standing,
        PlayerPose::Crouching => AvatarPose::Crouching,
        PlayerPose::Swimming => AvatarPose::Swimming,
        PlayerPose::FallFlying => AvatarPose::FallFlying,
        PlayerPose::SpinAttack => AvatarPose::SpinAttack,
        PlayerPose::Sleeping => AvatarPose::Sleeping,
        PlayerPose::Dying => AvatarPose::Dying,
    };
    let context = LocalAvatarContext {
        version: 1,
        handle: local_handle(world, epoch),
        world: world.to_string(),
        epoch: epoch.to_string(),
        position_previous: player.exact_old_position().to_array(),
        position: player.exact_position().to_array(),
        velocity_previous: avatar.delta_movement_on_previous_tick.as_dvec3().to_array(),
        velocity: player.exact_velocity().to_array(),
        body_yaw_previous_radians: f64::from(player.prev_body_yaw),
        body_yaw_radians: f64::from(player.body_yaw),
        head_yaw_previous_radians: f64::from(player.prev_head_yaw),
        head_yaw_radians: f64::from(player.head_yaw),
        pose,
        on_ground: player.on_ground,
        is_crouching: player.is_crouching_like_java(),
        is_fall_flying: player.is_fall_flying,
        cloak_previous: [avatar.x_cloak_o, avatar.y_cloak_o, avatar.z_cloak_o],
        cloak: [avatar.x_cloak, avatar.y_cloak, avatar.z_cloak],
        bob_previous: f64::from(avatar.bob_o),
        bob: f64::from(avatar.bob),
        walk_distance_previous: f64::from(avatar.walk_dist_o),
        walk_distance: f64::from(avatar.walk_dist),
        motion: Some(LocalAvatarMotion {
            player_yaw_radians: f64::from(player.yaw),
            head_pitch_radians: f64::from(player.pitch),
            is_under_water: player.is_under_water,
            is_visually_swimming: player.is_visually_swimming,
        }),
    };
    context.validate()?;
    Ok(context)
}
