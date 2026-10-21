use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_boat_model_tree() -> JavaModelPartTree {
    build_boat_model_tree_internal(false)
}

pub fn build_chest_boat_model_tree() -> JavaModelPartTree {
    build_boat_model_tree_internal(true)
}

fn build_boat_model_tree_internal(has_chest: bool) -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let mut root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, pi / 2.0, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, pi * 1.5, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, pi / 2.0, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, pi, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, pi / 16.0),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, pi, pi / 16.0),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );

    if has_chest {
        root = root
            .child(
                NamedPartSpec::new(
                    "chest_bottom",
                    PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -pi / 2.0, 0.0),
                )
                .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
            )
            .child(
                NamedPartSpec::new(
                    "chest_lid",
                    PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -pi / 2.0, 0.0),
                )
                .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
            )
            .child(
                NamedPartSpec::new(
                    "chest_lock",
                    PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -pi / 2.0, 0.0),
                )
                .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
            );
    }

    JavaModelPartTree::from_root(root)
}

pub fn build_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}

pub fn build_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t.clamp(0.0, 1.0) * (b - a)
}

fn animate_paddle(time: f32, side: usize, tree: &mut JavaModelPartTree, part_name: &str) {
    let pi = std::f32::consts::PI;
    let x_rot = lerp((-time).sin() * 0.5 + 0.5, -pi / 3.0, -pi / 12.0);
    let mut y_rot = lerp(((-time + 1.0).sin() + 1.0) * 0.5, -pi / 4.0, pi / 4.0);
    if side == 1 {
        y_rot = pi - y_rot;
    }
    if let Some(p) = tree.part_mut(part_name) {
        p.pose.rotation.x = x_rot;
        p.pose.rotation.y = y_rot;
    }
}

pub fn apply_boat_model_setup_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    animate_paddle(state.rowing_time_left, 0, tree, "left_paddle");
    animate_paddle(state.rowing_time_right, 1, tree, "right_paddle");
}

pub fn build_boat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_boat_model_tree();
    tree.reset_pose();
    apply_boat_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}

pub fn build_chest_boat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_chest_boat_model_tree();
    tree.reset_pose();
    apply_boat_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
