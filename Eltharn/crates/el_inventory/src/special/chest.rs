use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChestType {
    #[default]
    Single,
    Left,
    Right,
}

fn warn_invalid_chest_texture(texture: &str) {
    static WARNED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    WARNED.get_or_init(|| {
        eprintln!(
            "el_inventory: chest: invalid texture id {texture:?}; using \
             minecraft:entity/chest/normal"
        );
    });
}

pub struct ChestSpecialRenderer {
    pub texture: String,
    pub openness: f32,
    pub chest_type: ChestType,
}

impl SpecialModelRenderer for ChestSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let tex = [64.0, 64.0];
        let open_rot = -(self.openness * std::f32::consts::FRAC_PI_2);
        let (body_origin_x, body_width, lock_origin_x, lock_width, visible_faces) =
            match self.chest_type {
                ChestType::Single => (1.0, 14.0, 7.0, 2.0, [true; 6]),
                ChestType::Left => (0.0, 15.0, 0.0, 1.0, all_faces_except_west()),
                ChestType::Right => (1.0, 15.0, 15.0, 1.0, all_faces_except_east()),
            };
        vec![
            SpecialModelPart::new(
                SpecialPartPose::zero(),
                vec![
                    SpecialModelCube::new(
                        [0.0, 19.0],
                        [body_origin_x, 0.0, 1.0],
                        [body_width, 10.0, 14.0],
                        tex,
                    )
                    .visible_faces(visible_faces),
                ],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(0.0, 9.0, 1.0, open_rot, 0.0, 0.0),
                vec![
                    SpecialModelCube::new(
                        [0.0, 0.0],
                        [body_origin_x, 0.0, 0.0],
                        [body_width, 5.0, 14.0],
                        tex,
                    )
                    .visible_faces(visible_faces),
                ],
                Vec::new(),
            ),
            SpecialModelPart::new(
                SpecialPartPose::offset_and_rotation(0.0, 9.0, 1.0, open_rot, 0.0, 0.0),
                vec![
                    SpecialModelCube::new(
                        [0.0, 0.0],
                        [lock_origin_x, -2.0, 14.0],
                        [lock_width, 4.0, 1.0],
                        tex,
                    )
                    .visible_faces(visible_faces),
                ],
                Vec::new(),
            ),
        ]
    }

    fn get_texture(&self) -> String {
        let Some(loc) =
            el_core::resource_location::ResourceLocation::try_from_string(&self.texture)
        else {
            warn_invalid_chest_texture(&self.texture);
            return "minecraft:entity/chest/normal".to_string();
        };
        let name = loc.path.strip_prefix("entity/chest/").unwrap_or(&loc.path);
        format!("{}:entity/chest/{}", loc.namespace, name)
    }
}

fn all_faces_except_west() -> [bool; 6] {
    let mut faces = [true; 6];
    faces[4] = false;
    faces
}

fn all_faces_except_east() -> [bool; 6] {
    let mut faces = [true; 6];
    faces[5] = false;
    faces
}
