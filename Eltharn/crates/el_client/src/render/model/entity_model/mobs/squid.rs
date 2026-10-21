use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub const TENTACLE_NAMES: [&str; 8] = [
    "tentacle0",
    "tentacle1",
    "tentacle2",
    "tentacle3",
    "tentacle4",
    "tentacle5",
    "tentacle6",
    "tentacle7",
];

pub fn build_squid_model_tree() -> JavaModelPartTree {
    let mut root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("body", PartPose::offset(0.0, 8.0, 0.0)).cube(cube_grown(
            [0.0, 0.0],
            [-6.0, -8.0, -6.0],
            [12.0, 16.0, 12.0],
            0.02,
        )),
    );

    for (i, &name) in TENTACLE_NAMES.iter().enumerate() {
        let angle = i as f32 * std::f32::consts::PI * 2.0 / 8.0;
        let x = angle.cos() * 5.0;
        let z = angle.sin() * 5.0;
        let y_rot = i as f32 * std::f32::consts::PI * -2.0 / 8.0 + (std::f32::consts::PI / 2.0);
        root = root.child(
            NamedPartSpec::new(
                name,
                PartPose::offset_and_rotation(x, 15.0, z, 0.0, y_rot, 0.0),
            )
            .cube(cube([48.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 18.0, 2.0])),
        );
    }

    JavaModelPartTree::from_root(root)
}

pub fn build_squid_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_squid_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_squid_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    for &name in TENTACLE_NAMES.iter() {
        if let Some(t) = tree.part_mut(name) {
            t.pose.rotation.x = state.tentacle_angle;
        }
    }
}

pub fn build_squid_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_squid_model_tree();
    tree.reset_pose();
    apply_squid_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
