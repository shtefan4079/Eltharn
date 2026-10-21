use crate::special::{
    SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose, SpecialRenderLayer,
    pot_decoration_items,
};
use el_core::item::ItemStack;
use el_core::item::decorated_pot_pattern_asset;

pub struct DecoratedPotSpecialRenderer {}

impl SpecialModelRenderer for DecoratedPotSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let mut parts = decorated_pot_base_parts();
        parts.extend(decorated_pot_side_parts());
        parts
    }

    fn get_texture(&self) -> String {
        "minecraft:entity/decorated_pot/decorated_pot_base".to_string()
    }

    fn get_layers(&self, stack: Option<&ItemStack>, tint: [f32; 4]) -> Vec<SpecialRenderLayer> {
        let decorations = pot_decoration_items(stack);
        let side_parts = decorated_pot_side_parts();
        let side_textures = [
            decorated_pot_side_texture(decorations[0].as_deref()),
            decorated_pot_side_texture(decorations[1].as_deref()),
            decorated_pot_side_texture(decorations[2].as_deref()),
            decorated_pot_side_texture(decorations[3].as_deref()),
        ];

        let mut layers = vec![SpecialRenderLayer::new(
            self.get_texture(),
            decorated_pot_base_parts(),
            tint,
        )];

        for (part, texture) in side_parts.into_iter().zip(side_textures) {
            layers.push(SpecialRenderLayer::new(texture, vec![part], tint));
        }
        layers
    }
}

fn decorated_pot_base_parts() -> Vec<SpecialModelPart> {
    let base_tex = [32.0, 32.0];
    vec![
        SpecialModelPart::new(
            SpecialPartPose::offset_and_rotation(0.0, 37.0, 16.0, std::f32::consts::PI, 0.0, 0.0),
            vec![
                SpecialModelCube::new([0.0, 0.0], [4.0, 17.0, 4.0], [8.0, 3.0, 8.0], base_tex)
                    .grown(-0.1),
                SpecialModelCube::new([0.0, 5.0], [5.0, 20.0, 5.0], [6.0, 1.0, 6.0], base_tex)
                    .grown(0.2),
            ],
            Vec::new(),
        ),
        SpecialModelPart::new(
            SpecialPartPose::offset(1.0, 16.0, 1.0),
            vec![SpecialModelCube::new(
                [-14.0, 13.0],
                [0.0, 0.0, 0.0],
                [14.0, 0.0, 14.0],
                base_tex,
            )],
            Vec::new(),
        ),
        SpecialModelPart::new(
            SpecialPartPose::offset(1.0, 0.0, 1.0),
            vec![SpecialModelCube::new(
                [-14.0, 13.0],
                [0.0, 0.0, 0.0],
                [14.0, 0.0, 14.0],
                base_tex,
            )],
            Vec::new(),
        ),
    ]
}

fn decorated_pot_side_parts() -> Vec<SpecialModelPart> {
    let side_tex = [16.0, 16.0];
    let mut north_face = [false; 6];
    north_face[2] = true;
    let side_plane = || {
        SpecialModelCube::new([1.0, 0.0], [0.0, 0.0, 0.0], [14.0, 16.0, 0.0], side_tex)
            .visible_faces(north_face)
    };

    vec![
        SpecialModelPart::new(
            SpecialPartPose::offset_and_rotation(15.0, 16.0, 1.0, 0.0, 0.0, std::f32::consts::PI),
            vec![side_plane()],
            Vec::new(),
        ),
        SpecialModelPart::new(
            SpecialPartPose::offset_and_rotation(
                1.0,
                16.0,
                1.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                std::f32::consts::PI,
            ),
            vec![side_plane()],
            Vec::new(),
        ),
        SpecialModelPart::new(
            SpecialPartPose::offset_and_rotation(
                15.0,
                16.0,
                15.0,
                0.0,
                std::f32::consts::FRAC_PI_2,
                std::f32::consts::PI,
            ),
            vec![side_plane()],
            Vec::new(),
        ),
        SpecialModelPart::new(
            SpecialPartPose::offset_and_rotation(1.0, 16.0, 15.0, std::f32::consts::PI, 0.0, 0.0),
            vec![side_plane()],
            Vec::new(),
        ),
    ]
}

fn decorated_pot_side_texture(item_path: Option<&str>) -> String {
    let Some(item_path) = item_path else {
        return "minecraft:entity/decorated_pot/decorated_pot_side".to_string();
    };
    if item_path == "brick" {
        return "minecraft:entity/decorated_pot/decorated_pot_side".to_string();
    }
    let Some(pattern) = decorated_pot_pattern_asset(item_path) else {
        return "minecraft:entity/decorated_pot/decorated_pot_side".to_string();
    };
    format!("minecraft:entity/decorated_pot/{pattern}")
}
