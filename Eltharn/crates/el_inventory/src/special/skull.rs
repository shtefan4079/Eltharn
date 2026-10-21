use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct SkullSpecialRenderer {
    pub kind: String,
    pub texture: Option<String>,
    pub animation: f32,
}

impl SpecialModelRenderer for SkullSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let (texture_loc, crop_pixels) = self.get_texture_and_crop();

        if crop_pixels == Some([112.0, 30.0, 16.0, 16.0]) {
            return dragon_head_parts(self.animation);
        }
        if texture_loc.contains("piglin") {
            return piglin_head_parts(self.animation);
        }

        let tex = if texture_loc.contains("skeleton") || texture_loc.contains("creeper") {
            [64.0, 32.0]
        } else {
            [64.0, 64.0]
        };
        let mut children = Vec::new();
        if texture_loc.contains("player") || texture_loc.contains("zombie") {
            children.push(SpecialModelPart::new(
                SpecialPartPose::zero(),
                vec![
                    SpecialModelCube::new([32.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], tex)
                        .grown(0.25),
                ],
                Vec::new(),
            ));
        }

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
        self.get_texture_and_crop().0
    }

    fn get_crop_pixels(&self) -> Option<[f32; 4]> {
        self.get_texture_and_crop().1
    }
}

impl SkullSpecialRenderer {
    fn get_texture_and_crop(&self) -> (String, Option<[f32; 4]>) {
        if let Some(ref over_str) = self.texture {
            let loc = el_core::resource_location::ResourceLocation::from_string(over_str);
            let name = loc.path.strip_prefix("entity/").unwrap_or(&loc.path);
            (
                format!("{}:entity/{}", loc.namespace, name),
                Some([8.0, 8.0, 8.0, 8.0]),
            )
        } else {
            let (loc, crop) = match self.kind.as_str() {
                "skeleton" => (
                    el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "entity/skeleton/skeleton",
                    ),
                    Some([8.0, 8.0, 8.0, 8.0]),
                ),
                "wither_skeleton" => (
                    el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "entity/skeleton/wither_skeleton",
                    ),
                    Some([8.0, 8.0, 8.0, 8.0]),
                ),
                "zombie" => (
                    el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "entity/zombie/zombie",
                    ),
                    Some([8.0, 8.0, 8.0, 8.0]),
                ),
                "creeper" => (
                    el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "entity/creeper/creeper",
                    ),
                    Some([8.0, 8.0, 8.0, 8.0]),
                ),
                "piglin" => (
                    el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "entity/piglin/piglin",
                    ),
                    Some([8.0, 8.0, 8.0, 8.0]),
                ),
                "dragon" => (
                    el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "entity/enderdragon/dragon",
                    ),
                    Some([112.0, 30.0, 16.0, 16.0]),
                ),
                _ => (
                    el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "entity/player/wide/steve",
                    ),
                    Some([8.0, 8.0, 8.0, 8.0]),
                ),
            };
            (loc.to_string(), crop)
        }
    }
}

fn piglin_head_parts(animation: f32) -> Vec<SpecialModelPart> {
    let tex = [64.0, 64.0];
    let left_ear_z = -((animation * std::f32::consts::PI * 0.2 * 1.2).cos() + 2.5) * 0.2;
    let right_ear_z = ((animation * std::f32::consts::PI * 0.2).cos() + 2.5) * 0.2;
    vec![SpecialModelPart::new(
        SpecialPartPose::zero(),
        vec![
            SpecialModelCube::new([0.0, 0.0], [-5.0, -8.0, -4.0], [10.0, 8.0, 8.0], tex),
            SpecialModelCube::new([31.0, 1.0], [-2.0, -4.0, -5.0], [4.0, 4.0, 1.0], tex),
            SpecialModelCube::new([2.0, 4.0], [2.0, -2.0, -5.0], [1.0, 2.0, 1.0], tex),
            SpecialModelCube::new([2.0, 0.0], [-3.0, -2.0, -5.0], [1.0, 2.0, 1.0], tex),
        ],
        vec![
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(4.5, -6.0, 0.0, 0.0, 0.0, left_ear_z),
                vec![SpecialModelCube::new(
                    [51.0, 6.0],
                    [0.0, 0.0, -2.0],
                    [1.0, 5.0, 4.0],
                    tex,
                )],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(-4.5, -6.0, 0.0, 0.0, 0.0, right_ear_z),
                vec![SpecialModelCube::new(
                    [39.0, 6.0],
                    [-1.0, 0.0, -2.0],
                    [1.0, 5.0, 4.0],
                    tex,
                )],
                Vec::new(),
            ),
        ],
    )]
}

fn dragon_head_parts(animation: f32) -> Vec<SpecialModelPart> {
    let tex = [256.0, 256.0];
    let jaw_x_rot = ((animation * std::f32::consts::PI * 0.2).sin() + 1.0) * 0.2;
    vec![SpecialModelPart::new(
        SpecialPartPose::offset(0.0, -7.986666, 0.0).scaled(0.75),
        vec![
            SpecialModelCube::new([176.0, 44.0], [-6.0, -1.0, -24.0], [12.0, 5.0, 16.0], tex),
            SpecialModelCube::new([112.0, 30.0], [-8.0, -8.0, -10.0], [16.0, 16.0, 16.0], tex),
            SpecialModelCube::new([0.0, 0.0], [-5.0, -12.0, -4.0], [2.0, 4.0, 6.0], tex).mirrored(),
            SpecialModelCube::new([112.0, 0.0], [-5.0, -3.0, -22.0], [2.0, 2.0, 4.0], tex)
                .mirrored(),
            SpecialModelCube::new([0.0, 0.0], [3.0, -12.0, -4.0], [2.0, 4.0, 6.0], tex),
            SpecialModelCube::new([112.0, 0.0], [3.0, -3.0, -22.0], [2.0, 2.0, 4.0], tex),
        ],
        vec![SpecialModelPart::new(
            SpecialPartPose::offset_and_rotation(0.0, 4.0, -8.0, jaw_x_rot, 0.0, 0.0),
            vec![SpecialModelCube::new(
                [176.0, 65.0],
                [-6.0, 0.0, -16.0],
                [12.0, 4.0, 16.0],
                tex,
            )],
            Vec::new(),
        )],
    )]
}
