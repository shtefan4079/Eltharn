use crate::special::{
    SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose, SpecialRenderLayer,
    banner_pattern_layers, dye_color_rgb, multiply_color,
};
use el_core::item::ItemStack;

pub struct BannerSpecialRenderer {
    pub base_color: String,
    pub attachment: String,
}

impl SpecialModelRenderer for BannerSpecialRenderer {
    fn get_parts(&self, tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let mut parts = banner_body_parts(tint, self.is_standing());
        parts.extend(banner_flag_parts(tint, self.is_standing()));
        parts
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/banner/banner_base".to_string()
    }

    fn get_tint(&self) -> [f32; 4] {
        dye_color_rgb(&self.base_color).unwrap_or([1.0, 1.0, 1.0, 1.0])
    }

    fn get_layers(&self, stack: Option<&ItemStack>, tint: [f32; 4]) -> Vec<SpecialRenderLayer> {
        let base_color = dye_color_rgb(&self.base_color).unwrap_or([1.0, 1.0, 1.0, 1.0]);
        let standing = self.is_standing();
        let flag_parts = banner_flag_parts([1.0, 1.0, 1.0, 1.0], standing);
        let mut layers = vec![
            SpecialRenderLayer::new(
                self.get_texture(),
                {
                    let mut parts = banner_body_parts([1.0, 1.0, 1.0, 1.0], standing);
                    parts.extend(flag_parts.clone());
                    parts
                },
                tint,
            ),
            SpecialRenderLayer::new(
                "minecraft:entity/banner/base",
                flag_parts.clone(),
                multiply_color(tint, base_color),
            ),
        ];

        for pattern in banner_pattern_layers(stack).into_iter().take(16) {
            let color = dye_color_rgb(&pattern.color).unwrap_or([1.0, 1.0, 1.0, 1.0]);
            layers.push(SpecialRenderLayer::new(
                format!("minecraft:entity/banner/{}", pattern.pattern),
                flag_parts.clone(),
                multiply_color(tint, color),
            ));
        }
        layers
    }
}

impl BannerSpecialRenderer {
    fn is_standing(&self) -> bool {
        self.attachment != "wall"
    }
}

fn banner_body_parts(color: [f32; 4], standing: bool) -> Vec<SpecialModelPart> {
    let tex = [64.0, 64.0];
    let mut parts = Vec::new();

    if standing {
        parts.push(SpecialModelPart::new_colored(
            SpecialPartPose::zero(),
            vec![SpecialModelCube::new(
                [44.0, 0.0],
                [-1.0, -42.0, -1.0],
                [2.0, 42.0, 2.0],
                tex,
            )],
            Vec::new(),
            color,
        ));
    }

    parts.push(SpecialModelPart::new_colored(
        SpecialPartPose::zero(),
        vec![SpecialModelCube::new(
            [0.0, 42.0],
            [
                -10.0,
                if standing { -44.0 } else { -20.5 },
                if standing { -1.0 } else { 9.5 },
            ],
            [20.0, 2.0, 2.0],
            tex,
        )],
        Vec::new(),
        color,
    ));
    parts
}

fn banner_flag_parts(color: [f32; 4], standing: bool) -> Vec<SpecialModelPart> {
    let tex = [64.0, 64.0];
    vec![SpecialModelPart::new_colored(
        SpecialPartPose::offset_and_rotation(
            0.0,
            if standing { -44.0 } else { -20.5 },
            if standing { 0.0 } else { 10.5 },
            -0.0025 * std::f32::consts::PI,
            0.0,
            0.0,
        ),
        vec![SpecialModelCube::new(
            [0.0, 0.0],
            [-10.0, 0.0, -2.0],
            [20.0, 40.0, 1.0],
            tex,
        )],
        Vec::new(),
        color,
    )]
}
