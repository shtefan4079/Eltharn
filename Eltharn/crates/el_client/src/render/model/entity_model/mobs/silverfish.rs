use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub const SILVERFISH_SEGMENT_NAMES: [&str; 7] = [
    "segment0", "segment1", "segment2", "segment3", "segment4", "segment5", "segment6",
];

pub const SILVERFISH_LAYER_NAMES: [&str; 3] = ["layer0", "layer1", "layer2"];

pub fn build_silverfish_model_tree() -> JavaModelPartTree {
    let body_sizes: [[f32; 3]; 7] = [
        [3.0, 2.0, 2.0],
        [4.0, 3.0, 2.0],
        [6.0, 4.0, 3.0],
        [3.0, 3.0, 3.0],
        [2.0, 2.0, 3.0],
        [2.0, 1.0, 2.0],
        [1.0, 1.0, 2.0],
    ];

    let body_texs: [[f32; 2]; 7] = [
        [0.0, 0.0],
        [0.0, 4.0],
        [0.0, 9.0],
        [0.0, 16.0],
        [0.0, 22.0],
        [11.0, 0.0],
        [13.0, 4.0],
    ];

    let mut root = NamedPartSpec::new("root", PartPose::ZERO);
    let mut z_placement = [0.0_f32; 7];
    let mut placement = -3.5_f32;

    for i in 0..7 {
        let size = body_sizes[i];
        let tex = body_texs[i];
        root = root.child(
            NamedPartSpec::new(
                SILVERFISH_SEGMENT_NAMES[i],
                PartPose::offset(0.0, 24.0 - size[1], placement),
            )
            .cube(cube(
                tex,
                [-size[0] * 0.5, 0.0, -size[2] * 0.5],
                [size[0], size[1], size[2]],
            )),
        );
        z_placement[i] = placement;
        if i < 6 {
            placement += (size[2] + body_sizes[i + 1][2]) * 0.5;
        }
    }

    root = root
        .child(
            NamedPartSpec::new(
                SILVERFISH_LAYER_NAMES[0],
                PartPose::offset(0.0, 16.0, z_placement[2]),
            )
            .cube(cube(
                [20.0, 0.0],
                [-5.0, 0.0, -body_sizes[2][2] * 0.5],
                [10.0, 8.0, body_sizes[2][2]],
            )),
        )
        .child(
            NamedPartSpec::new(
                SILVERFISH_LAYER_NAMES[1],
                PartPose::offset(0.0, 20.0, z_placement[4]),
            )
            .cube(cube(
                [20.0, 11.0],
                [-3.0, 0.0, -body_sizes[4][2] * 0.5],
                [6.0, 4.0, body_sizes[4][2]],
            )),
        )
        .child(
            NamedPartSpec::new(
                SILVERFISH_LAYER_NAMES[2],
                PartPose::offset(0.0, 19.0, z_placement[1]),
            )
            .cube(cube(
                [20.0, 18.0],
                [-3.0, 0.0, -body_sizes[4][2] * 0.5],
                [6.0, 5.0, body_sizes[1][2]],
            )),
        );

    JavaModelPartTree::from_root(root)
}

pub fn build_silverfish_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_silverfish_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_silverfish_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let age = state.age_in_ticks;
    let mut seg_y_rot = [0.0_f32; 7];
    let mut seg_x = [0.0_f32; 7];

    for i in 0..7 {
        let diff = (i as i32 - 2).abs() as f32;
        let y_rot = (age * 0.9 + i as f32 * 0.15 * std::f32::consts::PI).cos()
            * std::f32::consts::PI
            * 0.05
            * (1.0 + diff);
        let x_off = (age * 0.9 + i as f32 * 0.15 * std::f32::consts::PI).sin()
            * std::f32::consts::PI
            * 0.2
            * diff;

        seg_y_rot[i] = y_rot;
        seg_x[i] = x_off;

        if let Some(s) = tree.part_mut(SILVERFISH_SEGMENT_NAMES[i]) {
            s.pose.rotation.y = y_rot;
            s.pose.offset.x = x_off;
        }
    }

    if let Some(l0) = tree.part_mut(SILVERFISH_LAYER_NAMES[0]) {
        l0.pose.rotation.y = seg_y_rot[2];
    }
    if let Some(l1) = tree.part_mut(SILVERFISH_LAYER_NAMES[1]) {
        l1.pose.rotation.y = seg_y_rot[4];
        l1.pose.offset.x = seg_x[4];
    }
    if let Some(l2) = tree.part_mut(SILVERFISH_LAYER_NAMES[2]) {
        l2.pose.rotation.y = seg_y_rot[1];
        l2.pose.offset.x = seg_x[1];
    }
}

pub fn build_silverfish_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_silverfish_model_tree();
    tree.reset_pose();
    apply_silverfish_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
