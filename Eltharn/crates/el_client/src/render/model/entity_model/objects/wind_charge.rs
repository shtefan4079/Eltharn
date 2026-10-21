use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_wind_charge_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("bone", PartPose::ZERO)
            .child(
                NamedPartSpec::new(
                    "wind",
                    PartPose::offset_and_rotation(0.0, 0.0, 0.0, 0.0, -0.7854, 0.0),
                )
                .cube(cube([15.0, 20.0], [-4.0, -1.0, -4.0], [8.0, 2.0, 8.0]))
                .cube(cube([0.0, 9.0], [-3.0, -2.0, -3.0], [6.0, 4.0, 6.0])),
            )
            .child(NamedPartSpec::new("wind_charge", PartPose::ZERO).cube(cube(
                [0.0, 0.0],
                [-2.0, -2.0, -2.0],
                [4.0, 4.0, 4.0],
            ))),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_wind_charge_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wind_charge_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_wind_charge_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    let age = state.age_in_ticks;

    if let Some(wc) = tree.part_mut("wind_charge") {
        wc.pose.rotation.y = -age * 16.0 * deg2rad;
    }
    if let Some(w) = tree.part_mut("wind") {
        w.pose.rotation.y = age * 16.0 * deg2rad;
    }
}

pub fn build_wind_charge_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wind_charge_model_tree();
    tree.reset_pose();
    apply_wind_charge_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
