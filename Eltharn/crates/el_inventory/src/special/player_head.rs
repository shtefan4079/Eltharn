use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct PlayerHeadSpecialRenderer {}

impl SpecialModelRenderer for PlayerHeadSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let tex = [64.0, 64.0];
        let mut children = Vec::new();
        children.push(SpecialModelPart::new(
            SpecialPartPose::zero(),
            vec![
                SpecialModelCube::new([32.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], tex)
                    .grown(0.25),
            ],
            Vec::new(),
        ));

        vec![SpecialModelPart::new(
            SpecialPartPose::zero(),
            vec![SpecialModelCube::new(
                [0.0, 0.0],
                [-4.0, -8.0, -4.0],
                [8.0, 8.0, 8.0],
                tex,
            )],
            children,
        )]
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/player/wide/steve".to_string()
    }

    fn get_crop_pixels(&self) -> Option<[f32; 4]> {
        Some([8.0, 8.0, 8.0, 8.0])
    }
}
