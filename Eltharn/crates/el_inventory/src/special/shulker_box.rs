use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct ShulkerBoxSpecialRenderer {
    pub texture: String,
    pub openness: f32,
}

impl SpecialModelRenderer for ShulkerBoxSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let tex = [64.0, 64.0];
        let bs = (0.5 + self.openness) * std::f32::consts::PI;
        let sin_bs = bs.sin();
        let q = -1.0 + sin_bs;
        let lid_y = 16.0 + sin_bs * 8.0;
        let lid_y_rot = if self.openness > 0.3 {
            q * q * q * q * std::f32::consts::PI * 0.125
        } else {
            0.0
        };
        vec![
            SpecialModelPart::new(
                SpecialPartPose::offset(0.0, 24.0, 0.0),
                vec![SpecialModelCube::new(
                    [0.0, 28.0],
                    [-8.0, -8.0, -8.0],
                    [16.0, 8.0, 16.0],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(0.0, lid_y, 0.0, 0.0, lid_y_rot, 0.0),
                vec![SpecialModelCube::new(
                    [0.0, 0.0],
                    [-8.0, -16.0, -8.0],
                    [16.0, 12.0, 16.0],
                    tex,
                )],
                Vec::new(),
            ),
        ]
    }

    fn get_texture(&self) -> String {
        let loc = el_core::resource_location::ResourceLocation::from_string(&self.texture);
        let name = loc
            .path
            .strip_prefix("entity/shulker/")
            .unwrap_or(&loc.path);
        format!("{}:entity/shulker/{}", loc.namespace, name)
    }
}
