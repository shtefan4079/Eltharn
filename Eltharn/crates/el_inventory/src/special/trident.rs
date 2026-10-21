use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct TridentSpecialRenderer {}

impl SpecialModelRenderer for TridentSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let tex = [32.0, 32.0];
        vec![SpecialModelPart::new(
            SpecialPartPose::zero(),
            vec![SpecialModelCube::new(
                [0.0, 6.0],
                [-0.5, 2.0, -0.5],
                [1.0, 25.0, 1.0],
                tex,
            )],
            vec![
                SpecialModelPart::new(
                    SpecialPartPose::zero(),
                    vec![SpecialModelCube::new(
                        [4.0, 0.0],
                        [-1.5, 0.0, -0.5],
                        [3.0, 2.0, 1.0],
                        tex,
                    )],
                    Vec::new(),
                ),
                SpecialModelPart::new(
                    SpecialPartPose::zero(),
                    vec![SpecialModelCube::new(
                        [4.0, 3.0],
                        [-2.5, -3.0, -0.5],
                        [1.0, 4.0, 1.0],
                        tex,
                    )],
                    Vec::new(),
                ),
                SpecialModelPart::new(
                    SpecialPartPose::zero(),
                    vec![SpecialModelCube::new(
                        [0.0, 0.0],
                        [-0.5, -4.0, -0.5],
                        [1.0, 4.0, 1.0],
                        tex,
                    )],
                    Vec::new(),
                ),
                SpecialModelPart::new(
                    SpecialPartPose::zero(),
                    vec![
                        SpecialModelCube::new([4.0, 3.0], [1.5, -3.0, -0.5], [1.0, 4.0, 1.0], tex)
                            .mirrored(),
                    ],
                    Vec::new(),
                ),
            ],
        )]
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/trident/trident".to_string()
    }
}
