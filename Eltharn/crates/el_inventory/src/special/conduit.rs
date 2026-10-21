use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct ConduitSpecialRenderer {}

impl SpecialModelRenderer for ConduitSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        vec![SpecialModelPart::new(
            SpecialPartPose::zero(),
            vec![SpecialModelCube::new(
                [0.0, 0.0],
                [-3.0, -3.0, -3.0],
                [6.0, 6.0, 6.0],
                [32.0, 16.0],
            )],
            Vec::new(),
        )]
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/conduit/base".to_string()
    }
}
