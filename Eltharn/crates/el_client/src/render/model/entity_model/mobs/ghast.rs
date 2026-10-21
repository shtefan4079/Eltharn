use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
use glam::Vec3;

pub fn build_ghast_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new(
            "root",
            PartPose {
                offset: Vec3::new(0.0, -84.056, 0.0),
                rotation: Vec3::splat(0.0),
                scale: Vec3::splat(4.5),
            },
        )
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 17.6, 0.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -8.0, -8.0],
                [16.0, 16.0, 16.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle0", PartPose::offset(-3.75, 24.6, -5.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle1", PartPose::offset(0.416667, 24.6, -3.333333))
                .cube(cube([0.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 13.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tentacle2", PartPose::offset(4.583333, 24.6, -1.666667))
                .cube(cube([0.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 9.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tentacle3", PartPose::offset(-6.25, 24.6, 0.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 11.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle4", PartPose::offset(-2.083333, 24.6, 1.666667))
                .cube(cube([0.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 11.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tentacle5", PartPose::offset(2.083333, 24.6, 3.333333)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 10.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle6", PartPose::offset(-3.75, 24.6, 5.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle7", PartPose::offset(0.416667, 24.6, 6.666667)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 9.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle8", PartPose::offset(4.583333, 24.6, 8.333333)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_ghast_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_ghast_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_ghast_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    const TENTACLE_NAMES: [&str; 9] = [
        "tentacle0",
        "tentacle1",
        "tentacle2",
        "tentacle3",
        "tentacle4",
        "tentacle5",
        "tentacle6",
        "tentacle7",
        "tentacle8",
    ];
    for (i, &name) in TENTACLE_NAMES.iter().enumerate() {
        if let Some(t) = tree.part_mut(name) {
            t.pose.rotation.x = 0.2 * (state.age_in_ticks * 0.3 + i as f32).sin() + 0.4;
        }
    }
}

pub fn build_ghast_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_ghast_model_tree();
    tree.reset_pose();
    apply_ghast_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
