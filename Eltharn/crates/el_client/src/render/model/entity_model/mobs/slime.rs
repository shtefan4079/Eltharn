use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_slime_outer_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("cube", PartPose::ZERO).cube(cube(
            [0.0, 0.0],
            [-4.0, 16.0, -4.0],
            [8.0, 8.0, 8.0],
        )),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_slime_inner_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(NamedPartSpec::new("cube", PartPose::ZERO).cube(cube(
            [0.0, 16.0],
            [-3.0, 17.0, -3.0],
            [6.0, 6.0, 6.0],
        )))
        .child(NamedPartSpec::new("right_eye", PartPose::ZERO).cube(cube(
            [32.0, 0.0],
            [-3.25, 18.0, -3.5],
            [2.0, 2.0, 2.0],
        )))
        .child(NamedPartSpec::new("left_eye", PartPose::ZERO).cube(cube(
            [32.0, 4.0],
            [1.25, 18.0, -3.5],
            [2.0, 2.0, 2.0],
        )))
        .child(NamedPartSpec::new("mouth", PartPose::ZERO).cube(cube(
            [32.0, 8.0],
            [0.0, 21.0, -3.5],
            [1.0, 1.0, 1.0],
        )));

    JavaModelPartTree::from_root(root)
}

pub fn build_slime_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let (mut inner_v, mut inner_i) = build_slime_inner_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    });
    let (outer_v, outer_i) = build_slime_outer_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    });
    let offset = inner_v.len() as u32;
    inner_v.extend(outer_v);
    inner_i.extend(outer_i.into_iter().map(|idx| idx + offset));
    (inner_v, inner_i)
}

pub fn build_magma_cube_model_tree() -> JavaModelPartTree {
    let mut root = NamedPartSpec::new("root", PartPose::ZERO);

    const MAGMA_CUBE_NAMES: [&str; 8] = [
        "cube0", "cube1", "cube2", "cube3", "cube4", "cube5", "cube6", "cube7",
    ];

    for i in 0..8 {
        let mut u = 0.0_f32;
        let mut v = 0.0_f32;
        if i > 0 && i < 4 {
            v += 9.0 * i as f32;
        } else if i > 3 {
            u = 32.0;
            v += 9.0 * i as f32 - 36.0;
        }

        root = root.child(
            NamedPartSpec::new(MAGMA_CUBE_NAMES[i], PartPose::ZERO).cube(cube(
                [u, v],
                [-4.0, 16.0 + i as f32, -4.0],
                [8.0, 1.0, 8.0],
            )),
        );
    }

    root = root.child(NamedPartSpec::new("inside_cube", PartPose::ZERO).cube(cube(
        [24.0, 40.0],
        [-2.0, 18.0, -2.0],
        [4.0, 4.0, 4.0],
    )));

    JavaModelPartTree::from_root(root)
}

pub fn build_magma_cube_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_magma_cube_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}

pub fn apply_magma_cube_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    const MAGMA_CUBE_NAMES: [&str; 8] = [
        "cube0", "cube1", "cube2", "cube3", "cube4", "cube5", "cube6", "cube7",
    ];
    let squish = state.squish.max(0.0);
    for (i, &name) in MAGMA_CUBE_NAMES.iter().enumerate() {
        if let Some(c) = tree.part_mut(name) {
            c.pose.offset.y = -((4 - i as i32) as f32) * squish * 1.7;
        }
    }
}

pub fn build_magma_cube_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_magma_cube_model_tree();
    tree.reset_pose();
    apply_magma_cube_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
