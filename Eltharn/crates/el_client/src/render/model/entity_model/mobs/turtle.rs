use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_turtle_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 19.0, -10.0)).cube(cube(
                    [3.0, 0.0],
                    [-3.0, -1.0, -3.0],
                    [6.0, 5.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 11.0, -10.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([7.0, 37.0], [-9.5, 3.0, -10.0], [19.0, 20.0, 6.0]))
                .cube(cube([31.0, 1.0], [-5.5, 3.0, -13.0], [11.0, 18.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "egg_belly",
                    PartPose::offset_and_rotation(0.0, 11.0, -10.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([70.0, 33.0], [-4.5, 3.0, -14.0], [9.0, 18.0, 1.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.5, 22.0, 11.0))
                    .cube(cube([1.0, 23.0], [-2.0, 0.0, 0.0], [4.0, 1.0, 10.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.5, 22.0, 11.0)).cube(cube(
                    [1.0, 12.0],
                    [-2.0, 0.0, 0.0],
                    [4.0, 1.0, 10.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-5.0, 21.0, -4.0))
                    .cube(cube([27.0, 30.0], [-13.0, 0.0, -2.0], [13.0, 1.0, 5.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(5.0, 21.0, -4.0)).cube(cube(
                    [27.0, 24.0],
                    [0.0, 0.0, -2.0],
                    [13.0, 1.0, 5.0],
                )),
            );

    JavaModelPartTree::from_root(root)
}

pub fn build_turtle_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_turtle_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}

pub fn apply_turtle_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;

    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    }

    if let Some(eb) = tree.part_mut("egg_belly") {
        eb.visible = state.has_egg;
        if eb.visible {
            if let Some(r) = tree.part_mut("root") {
                r.pose.offset.y -= 1.0;
            }
        }
    }

    if state.is_on_land {
        let lay_egg = if state.is_laying_egg { 4.0 } else { 1.0 };
        let lay_egg_amplitude = if state.is_laying_egg { 2.0 } else { 1.0 };
        let swing_pos = anim_pos * 5.0;
        let front_swing = (lay_egg * swing_pos).cos();
        let hind_swing = swing_pos.cos();
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.y = -front_swing * 8.0 * anim_speed * lay_egg_amplitude;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.y = front_swing * 8.0 * anim_speed * lay_egg_amplitude;
        }
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.y = -hind_swing * 3.0 * anim_speed;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.y = hind_swing * 3.0 * anim_speed;
        }
    } else {
        let swing_scale = 0.5 * anim_speed;
        let swing = (anim_pos * 0.6662 * 0.6).cos() * swing_scale;
        if let Some(l) = tree.part_mut("right_hind_leg") {
            l.pose.rotation.x = swing;
        }
        if let Some(l) = tree.part_mut("left_hind_leg") {
            l.pose.rotation.x = -swing;
        }
        if let Some(l) = tree.part_mut("right_front_leg") {
            l.pose.rotation.z = -swing;
        }
        if let Some(l) = tree.part_mut("left_front_leg") {
            l.pose.rotation.z = swing;
        }
    }
}

pub fn build_turtle_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_turtle_model_tree();
    tree.reset_pose();
    apply_turtle_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
