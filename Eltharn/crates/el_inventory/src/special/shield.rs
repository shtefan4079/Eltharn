use crate::special::{
    SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose, SpecialRenderLayer,
    banner_pattern_layers, dye_color_rgb, stack_dye_color,
};
use el_core::item::ItemStack;

pub struct ShieldSpecialRenderer {}

impl SpecialModelRenderer for ShieldSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        shield_model_parts()
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/shield/shield_base_nopattern".to_string()
    }

    fn get_layers(&self, stack: Option<&ItemStack>, tint: [f32; 4]) -> Vec<SpecialRenderLayer> {
        let patterns = banner_pattern_layers(stack);
        let base_color = stack_dye_color(stack, "minecraft:base_color");
        let has_patterns = !patterns.is_empty() || base_color.is_some();
        let parts = shield_model_parts();
        let mut layers = vec![SpecialRenderLayer::new(
            if has_patterns {
                "minecraft:entity/shield/shield_base"
            } else {
                "minecraft:entity/shield/shield_base_nopattern"
            },
            parts.clone(),
            tint,
        )];

        if has_patterns {
            let base_tint = base_color
                .as_deref()
                .and_then(dye_color_rgb)
                .unwrap_or([1.0, 1.0, 1.0, 1.0]);
            layers.push(SpecialRenderLayer::new(
                "minecraft:entity/shield/base",
                parts.clone(),
                [
                    tint[0] * base_tint[0],
                    tint[1] * base_tint[1],
                    tint[2] * base_tint[2],
                    tint[3] * base_tint[3],
                ],
            ));

            for pattern in patterns.into_iter().take(16) {
                let color = dye_color_rgb(&pattern.color).unwrap_or([1.0, 1.0, 1.0, 1.0]);
                layers.push(SpecialRenderLayer::new(
                    format!("minecraft:entity/shield/{}", pattern.pattern),
                    parts.clone(),
                    [
                        tint[0] * color[0],
                        tint[1] * color[1],
                        tint[2] * color[2],
                        tint[3] * color[3],
                    ],
                ));
            }
        }
        layers
    }
}

fn shield_model_parts() -> Vec<SpecialModelPart> {
    let tex = [64.0, 64.0];
    vec![
        SpecialModelPart::new(
            SpecialPartPose::zero(),
            vec![SpecialModelCube::new(
                [0.0, 0.0],
                [-6.0, -11.0, -2.0],
                [12.0, 22.0, 1.0],
                tex,
            )],
            Vec::new(),
        ),
        SpecialModelPart::new(
            SpecialPartPose::zero(),
            vec![SpecialModelCube::new(
                [26.0, 0.0],
                [-1.0, -3.0, -1.0],
                [2.0, 6.0, 6.0],
                tex,
            )],
            Vec::new(),
        ),
    ]
}
