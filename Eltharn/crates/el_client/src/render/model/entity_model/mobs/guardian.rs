use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
const SPIKE_X_ROT: [f32; 12] = [
    1.75, 0.25, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5, 1.25, 0.75, 0.0, 0.0,
];
const SPIKE_Y_ROT: [f32; 12] = [
    0.0, 0.0, 0.0, 0.0, 0.25, 1.75, 1.25, 0.75, 0.0, 0.0, 0.0, 0.0,
];
const SPIKE_Z_ROT: [f32; 12] = [
    0.0, 0.0, 0.25, 1.75, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.75, 1.25,
];
const SPIKE_X: [f32; 12] = [
    0.0, 0.0, 8.0, -8.0, -8.0, 8.0, 8.0, -8.0, 0.0, 0.0, 8.0, -8.0,
];
const SPIKE_Y: [f32; 12] = [
    -8.0, -8.0, -8.0, -8.0, 0.0, 0.0, 0.0, 0.0, 8.0, 8.0, 8.0, 8.0,
];
const SPIKE_Z: [f32; 12] = [
    8.0, -8.0, 0.0, 0.0, -8.0, -8.0, 8.0, 8.0, 8.0, -8.0, 0.0, 0.0,
];
const SPIKE_NAMES: [&str; 12] = [
    "spike0", "spike1", "spike2", "spike3", "spike4", "spike5", "spike6", "spike7", "spike8",
    "spike9", "spike10", "spike11",
];
const ELDER_GUARDIAN_SCALE: f32 = 2.35;
fn build_guardian_head() -> NamedPartSpec {
    let mut head = NamedPartSpec::new("head", PartPose::ZERO)
        .cube(cube([0.0, 0.0], [-6.0, 10.0, -8.0], [12.0, 12.0, 16.0]))
        .cube(cube([0.0, 28.0], [-8.0, 10.0, -6.0], [2.0, 12.0, 12.0]))
        .cube(cube_mirrored(
            [0.0, 28.0],
            [6.0, 10.0, -6.0],
            [2.0, 12.0, 12.0],
        ))
        .cube(cube([16.0, 40.0], [-6.0, 8.0, -6.0], [12.0, 2.0, 12.0]))
        .cube(cube([16.0, 40.0], [-6.0, 22.0, -6.0], [12.0, 2.0, 12.0]));
    for (i, &name) in SPIKE_NAMES.iter().enumerate() {
        let offset = 1.0 + (i as f32).cos() * 0.01;
        let x = SPIKE_X[i] * offset;
        let y = 16.0 + SPIKE_Y[i] * offset;
        let z = SPIKE_Z[i] * offset;
        let x_rot = std::f32::consts::PI * SPIKE_X_ROT[i];
        let y_rot = std::f32::consts::PI * SPIKE_Y_ROT[i];
        let z_rot = std::f32::consts::PI * SPIKE_Z_ROT[i];
        head = head.child(
            NamedPartSpec::new(
                name,
                PartPose::offset_and_rotation(x, y, z, x_rot, y_rot, z_rot),
            )
            .cube(cube([0.0, 0.0], [-1.0, -4.5, -1.0], [2.0, 9.0, 2.0])),
        );
    }
    head = head
        .child(
            NamedPartSpec::new("eye", PartPose::offset(0.0, 0.0, -8.25)).cube(cube(
                [8.0, 0.0],
                [-1.0, 15.0, 0.0],
                [2.0, 2.0, 1.0],
            )),
        )
        .child(build_guardian_tail());
    head
}
fn build_guardian_tail() -> NamedPartSpec {
    let tail2 = NamedPartSpec::new("tail2", PartPose::offset(0.5, 0.5, 6.0))
        .cube(cube([41.0, 32.0], [0.0, 14.0, 0.0], [2.0, 2.0, 6.0]))
        .cube(cube([25.0, 19.0], [1.0, 10.5, 3.0], [1.0, 9.0, 9.0]));
    let tail1 = NamedPartSpec::new("tail1", PartPose::offset(-1.5, 0.5, 14.0))
        .cube(cube([0.0, 54.0], [0.0, 14.0, 0.0], [3.0, 3.0, 7.0]))
        .child(tail2);
    NamedPartSpec::new("tail0", PartPose::ZERO)
        .cube(cube([40.0, 0.0], [-2.0, 14.0, 7.0], [4.0, 4.0, 8.0]))
        .child(tail1)
}
pub fn build_elder_guardian_model_tree() -> JavaModelPartTree {
    let mut root = NamedPartSpec::new("root", PartPose::ZERO).child(build_guardian_head());
    let y_offset = 24.016 * (1.0 - ELDER_GUARDIAN_SCALE);
    root.pose = root
        .pose
        .scaled(ELDER_GUARDIAN_SCALE)
        .translated(0.0, y_offset, 0.0);
    JavaModelPartTree::from_root(root)
}
pub fn build_elder_guardian_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_elder_guardian_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_guardian_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(build_guardian_head());
    JavaModelPartTree::from_root(root)
}
pub fn build_guardian_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_guardian_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_guardian_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let deg2rad = std::f32::consts::PI / 180.0;
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * deg2rad;
        h.pose.rotation.x = state.x_rot_degrees * deg2rad;
    }
    let withdrawal = (1.0 - state.spikes_animation) * 0.55;
    for (i, &name) in SPIKE_NAMES.iter().enumerate() {
        let offset = 1.0 + (state.age_in_ticks * 1.5 + i as f32).cos() * 0.01 - withdrawal;
        if let Some(s) = tree.part_mut(name) {
            s.pose.offset.x = SPIKE_X[i] * offset;
            s.pose.offset.y = 16.0 + SPIKE_Y[i] * offset;
            s.pose.offset.z = SPIKE_Z[i] * offset;
        }
    }
    if let Some(e) = tree.part_mut("eye") {
        e.visible = true;
    }
    let swim = state.tail_animation;
    if let Some(t0) = tree.part_mut("tail0") {
        t0.pose.rotation.y = swim.sin() * std::f32::consts::PI * 0.05;
    }
    if let Some(t1) = tree.part_mut("tail1") {
        t1.pose.rotation.y = swim.sin() * std::f32::consts::PI * 0.1;
    }
    if let Some(t2) = tree.part_mut("tail2") {
        t2.pose.rotation.y = swim.sin() * std::f32::consts::PI * 0.15;
    }
}
pub fn build_guardian_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_guardian_model_tree();
    tree.reset_pose();
    apply_guardian_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
