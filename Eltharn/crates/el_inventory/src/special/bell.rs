use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct BellSpecialRenderer {}

impl SpecialModelRenderer for BellSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let tex = [32.0, 32.0];
        vec![SpecialModelPart::new(
            SpecialPartPose::offset(8.0, 12.0, 8.0),
            vec![SpecialModelCube::new(
                [0.0, 0.0],
                [-3.0, -6.0, -3.0],
                [6.0, 7.0, 6.0],
                tex,
            )],
            vec![SpecialModelPart::new(
                SpecialPartPose::offset(-8.0, -12.0, -8.0),
                vec![SpecialModelCube::new(
                    [0.0, 13.0],
                    [4.0, 4.0, 4.0],
                    [8.0, 2.0, 8.0],
                    tex,
                )],
                Vec::new(),
            )],
        )]
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/bell/bell_body".to_string()
    }
}
