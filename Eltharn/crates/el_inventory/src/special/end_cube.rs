use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct EndCubeSpecialRenderer {
    pub effect: String,
}

impl SpecialModelRenderer for EndCubeSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let tex = [64.0, 64.0];
        vec![SpecialModelPart::new(
            SpecialPartPose::zero(),
            vec![SpecialModelCube::new(
                [0.0, 0.0],
                [0.0, 0.0, 0.0],
                [16.0, 16.0, 16.0],
                tex,
            )],
            Vec::new(),
        )]
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/end_portal/end_portal".to_string()
    }
}
