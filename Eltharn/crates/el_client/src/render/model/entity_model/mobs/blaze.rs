use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub const BLAZE_PART_NAMES: [&str; 12] = [
    "part0", "part1", "part2", "part3", "part4", "part5", "part6", "part7", "part8", "part9",
    "part10", "part11",
];

pub fn build_blaze_model_tree() -> JavaModelPartTree {
    let mut root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("head", PartPose::ZERO).cube(cube(
            [0.0, 0.0],
            [-4.0, -4.0, -4.0],
            [8.0, 8.0, 8.0],
        )),
    );

    let rod_spec = cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0]);

    let mut angle = 0.0_f32;
    for i in 0..4 {
        let x = angle.cos() * 9.0;
        let y = -2.0 + (i as f32 * 2.0 * 0.25).cos();
        let z = angle.sin() * 9.0;
        root = root.child(
            NamedPartSpec::new(BLAZE_PART_NAMES[i], PartPose::offset(x, y, z))
                .cube(rod_spec.clone()),
        );
        angle += 1.0;
    }

    angle = std::f32::consts::PI / 4.0;
    for i in 4..8 {
        let x = angle.cos() * 7.0;
        let y = 2.0 + (i as f32 * 2.0 * 0.25).cos();
        let z = angle.sin() * 7.0;
        root = root.child(
            NamedPartSpec::new(BLAZE_PART_NAMES[i], PartPose::offset(x, y, z))
                .cube(rod_spec.clone()),
        );
        angle += 1.0;
    }

    angle = 0.47123894;
    for i in 8..12 {
        let x = angle.cos() * 5.0;
        let y = 11.0 + (i as f32 * 1.5 * 0.5).cos();
        let z = angle.sin() * 5.0;
        root = root.child(
            NamedPartSpec::new(BLAZE_PART_NAMES[i], PartPose::offset(x, y, z))
                .cube(rod_spec.clone()),
        );
        angle += 1.0;
    }

    JavaModelPartTree::from_root(root)
}

pub fn build_blaze_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_blaze_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_blaze_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    }

    let age = state.age_in_ticks;

    let mut angle = age * std::f32::consts::PI * -0.1;
    for i in 0..4 {
        if let Some(p) = tree.part_mut(BLAZE_PART_NAMES[i]) {
            p.pose.offset.y = -2.0 + ((i as f32 * 2.0 + age) * 0.25).cos();
            p.pose.offset.x = angle.cos() * 9.0;
            p.pose.offset.z = angle.sin() * 9.0;
        }
        angle += 1.0;
    }

    angle = (std::f32::consts::PI / 4.0) + age * std::f32::consts::PI * 0.03;
    for i in 4..8 {
        if let Some(p) = tree.part_mut(BLAZE_PART_NAMES[i]) {
            p.pose.offset.y = 2.0 + ((i as f32 * 2.0 + age) * 0.25).cos();
            p.pose.offset.x = angle.cos() * 7.0;
            p.pose.offset.z = angle.sin() * 7.0;
        }
        angle += 1.0;
    }

    angle = 0.47123894 + age * std::f32::consts::PI * -0.05;
    for i in 8..12 {
        if let Some(p) = tree.part_mut(BLAZE_PART_NAMES[i]) {
            p.pose.offset.y = 11.0 + ((i as f32 * 1.5 + age) * 0.5).cos();
            p.pose.offset.x = angle.cos() * 5.0;
            p.pose.offset.z = angle.sin() * 5.0;
        }
        angle += 1.0;
    }
}

pub fn build_blaze_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_blaze_model_tree();
    tree.reset_pose();
    apply_blaze_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
