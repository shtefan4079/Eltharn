use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;
const BABY_TRANSFORMER: f32 = 0.2375;
const BODY_SQUEEZE: f32 = 0.9375;
const SCALE_Y_OFFSET: f32 = 24.016;
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
fn build_happy_ghast_layer(is_baby: bool) -> NamedPartSpec {
    let body_offsets: [[f32; 3]; 9] = [
        [-3.75, 7.0, -5.0],
        [1.25, 7.0, -5.0],
        [6.25, 7.0, -5.0],
        [-6.25, 7.0, 0.0],
        [-1.25, 7.0, 0.0],
        [3.75, 7.0, 0.0],
        [-3.75, 7.0, 5.0],
        [1.25, 7.0, 5.0],
        [6.25, 7.0, 5.0],
    ];
    let body_sizes: [[f32; 3]; 9] = [
        [2.0, 5.0, 2.0],
        [2.0, 7.0, 2.0],
        [2.0, 4.0, 2.0],
        [2.0, 5.0, 2.0],
        [2.0, 5.0, 2.0],
        [2.0, 7.0, 2.0],
        [2.0, 8.0, 2.0],
        [2.0, 8.0, 2.0],
        [2.0, 5.0, 2.0],
    ];
    let mut body = NamedPartSpec::new("body", PartPose::offset(0.0, 16.0, 0.0)).cube(cube(
        [0.0, 0.0],
        [-8.0, -8.0, -8.0],
        [16.0, 16.0, 16.0],
    ));
    if is_baby {
        body = body.child(
            NamedPartSpec::new("inner_body", PartPose::offset(0.0, 8.0, 0.0)).cube(cube_grown(
                [0.0, 32.0],
                [-8.0, -16.0, -8.0],
                [16.0, 16.0, 16.0],
                -0.5,
            )),
        );
    }
    for i in 0..9 {
        body = body.child(
            NamedPartSpec::new(
                TENTACLE_NAMES[i],
                PartPose::offset(body_offsets[i][0], body_offsets[i][1], body_offsets[i][2]),
            )
            .cube(cube([0.0, 0.0], [-1.0, 0.0, -1.0], body_sizes[i])),
        );
    }
    let mut root = NamedPartSpec::new("root", PartPose::ZERO).child(body);
    let mut factor = 4.0;
    if is_baby {
        factor *= BABY_TRANSFORMER;
    }
    root.pose = root
        .pose
        .scaled(factor)
        .translated(0.0, SCALE_Y_OFFSET * (1.0 - factor), 0.0);
    root
}
pub fn build_happy_ghast_model_tree(is_baby: bool) -> JavaModelPartTree {
    JavaModelPartTree::from_root(build_happy_ghast_layer(is_baby))
}
pub fn build_happy_ghast_mesh(is_baby: bool) -> (Vec<Vertex>, Vec<u32>) {
    build_happy_ghast_model_tree(is_baby).mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_happy_ghast_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if !state.holds_body_item {
        if let Some(b) = tree.part_mut("body") {
            b.pose.scale = glam::Vec3::splat(BODY_SQUEEZE);
        }
    }
    for (i, &name) in TENTACLE_NAMES.iter().enumerate() {
        if let Some(t) = tree.part_mut(name) {
            t.pose.rotation.x = 0.2 * (state.age_in_ticks * 0.3 + i as f32).sin() + 0.4;
        }
    }
}
pub fn build_happy_ghast_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_happy_ghast_model_tree(state.is_baby);
    tree.reset_pose();
    apply_happy_ghast_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
