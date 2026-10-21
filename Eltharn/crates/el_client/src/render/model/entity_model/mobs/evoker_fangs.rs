use super::super::tree::*;
use super::super::types::*;
use crate::render::baked_model::Vertex;

pub fn build_evoker_fangs_model_tree() -> JavaModelPartTree {
    let pi = std::f32::consts::PI;
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("base", PartPose::offset(-5.0, 24.0, -5.0))
            .cube(cube([0.0, 0.0], [0.0, 0.0, 0.0], [10.0, 12.0, 10.0]))
            .child(
                NamedPartSpec::new(
                    "upper_jaw",
                    PartPose::offset_and_rotation(6.5, 0.0, 1.0, 0.0, 0.0, 2.042035),
                )
                .cube(cube([40.0, 0.0], [0.0, 0.0, 0.0], [4.0, 14.0, 8.0])),
            )
            .child(
                NamedPartSpec::new(
                    "lower_jaw",
                    PartPose::offset_and_rotation(3.5, 0.0, 9.0, 0.0, pi, 4.2411504),
                )
                .cube(cube([40.0, 0.0], [0.0, 0.0, 0.0], [4.0, 14.0, 8.0])),
            ),
    );

    JavaModelPartTree::from_root(root)
}

pub fn build_evoker_fangs_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_evoker_fangs_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}

pub fn apply_evoker_fangs_model_setup_anim(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let pi = std::f32::consts::PI;
    let bite_progress = state.bite_progress;
    let mut bite_amount = (bite_progress * 2.0).min(1.0);
    bite_amount = 1.0 - bite_amount * bite_amount * bite_amount;

    if let Some(uj) = tree.part_mut("upper_jaw") {
        uj.pose.rotation.z = pi - bite_amount * 0.35 * pi;
    }
    if let Some(lj) = tree.part_mut("lower_jaw") {
        lj.pose.rotation.z = pi + bite_amount * 0.35 * pi;
    }
}

pub fn build_evoker_fangs_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_evoker_fangs_model_tree();
    tree.reset_pose();
    apply_evoker_fangs_model_setup_anim(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
