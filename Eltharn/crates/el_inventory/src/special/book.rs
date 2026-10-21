use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct BookSpecialRenderer {
    pub open_angle: f32,
    pub page1: f32,
    pub page2: f32,
}

impl SpecialModelRenderer for BookSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let openness = self.open_angle * (std::f32::consts::PI / 180.0);
        let tex = [64.0, 32.0];
        let sin_open = openness.sin();
        vec![
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(
                    0.0,
                    0.0,
                    -1.0,
                    0.0,
                    std::f32::consts::PI + openness,
                    0.0,
                ),
                vec![SpecialModelCube::new(
                    [0.0, 0.0],
                    [-6.0, -5.0, -0.005],
                    [6.0, 10.0, 0.005],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(0.0, 0.0, 1.0, 0.0, -openness, 0.0),
                vec![SpecialModelCube::new(
                    [16.0, 0.0],
                    [0.0, -5.0, -0.005],
                    [6.0, 10.0, 0.005],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    std::f32::consts::FRAC_PI_2,
                    0.0,
                ),
                vec![SpecialModelCube::new(
                    [12.0, 0.0],
                    [-1.0, -5.0, 0.0],
                    [2.0, 10.0, 0.005],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(sin_open, 0.0, 0.0, 0.0, openness, 0.0),
                vec![SpecialModelCube::new(
                    [0.0, 10.0],
                    [0.0, -4.0, -0.99],
                    [5.0, 8.0, 1.0],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(sin_open, 0.0, 0.0, 0.0, -openness, 0.0),
                vec![SpecialModelCube::new(
                    [12.0, 10.0],
                    [0.0, -4.0, -0.01],
                    [5.0, 8.0, 1.0],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(
                    sin_open,
                    0.0,
                    0.0,
                    0.0,
                    openness - openness * 2.0 * self.page1,
                    0.0,
                ),
                vec![SpecialModelCube::new(
                    [24.0, 10.0],
                    [0.0, -4.0, 0.0],
                    [5.0, 8.0, 0.005],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(
                    sin_open,
                    0.0,
                    0.0,
                    0.0,
                    openness - openness * 2.0 * self.page2,
                    0.0,
                ),
                vec![SpecialModelCube::new(
                    [24.0, 10.0],
                    [0.0, -4.0, 0.0],
                    [5.0, 8.0, 0.005],
                    tex,
                )],
                Vec::new(),
            ),
        ]
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/enchantment/enchanting_table_book".to_string()
    }
}
