use crate::special::{SpecialModelCube, SpecialModelPart, SpecialModelRenderer, SpecialPartPose};

pub struct CopperGolemStatueSpecialRenderer {
    pub texture: String,
    pub pose: String,
}

impl SpecialModelRenderer for CopperGolemStatueSpecialRenderer {
    fn get_parts(&self, _tint: [f32; 4]) -> Vec<SpecialModelPart> {
        let tex = [64.0, 64.0];
        vec![SpecialModelPart::new(
            SpecialPartPose::offset(0.0, 24.0, 0.0),
            Vec::new(),
            vec![
                SpecialModelPart::new(
                    SpecialPartPose::offset(0.0, -5.0, 0.0),
                    vec![SpecialModelCube::new(
                        [0.0, 15.0],
                        [-4.0, -6.0, -3.0],
                        [8.0, 6.0, 6.0],
                        tex,
                    )],
                    vec![
                        SpecialModelPart::new(
                            SpecialPartPose::offset(0.0, -6.0, 0.0),
                            vec![
                                SpecialModelCube::new(
                                    [0.0, 0.0],
                                    [-4.0, -5.0, -5.0],
                                    [8.0, 5.0, 10.0],
                                    tex,
                                )
                                .grown(0.015),
                                SpecialModelCube::new(
                                    [56.0, 0.0],
                                    [-1.0, -2.0, -6.0],
                                    [2.0, 3.0, 2.0],
                                    tex,
                                ),
                                SpecialModelCube::new(
                                    [37.0, 8.0],
                                    [-1.0, -9.0, -1.0],
                                    [2.0, 4.0, 2.0],
                                    tex,
                                )
                                .grown(-0.015),
                                SpecialModelCube::new(
                                    [37.0, 0.0],
                                    [-2.0, -13.0, -2.0],
                                    [4.0, 4.0, 4.0],
                                    tex,
                                )
                                .grown(-0.015),
                            ],
                            Vec::new(),
                        ),
                        SpecialModelPart::new(
                            SpecialPartPose::offset(-4.0, -6.0, 0.0),
                            vec![SpecialModelCube::new(
                                [36.0, 16.0],
                                [-3.0, -1.0, -2.0],
                                [3.0, 10.0, 4.0],
                                tex,
                            )],
                            Vec::new(),
                        ),
                        SpecialModelPart::new(
                            SpecialPartPose::offset(4.0, -6.0, 0.0),
                            vec![SpecialModelCube::new(
                                [50.0, 16.0],
                                [0.0, -1.0, -2.0],
                                [3.0, 10.0, 4.0],
                                tex,
                            )],
                            Vec::new(),
                        ),
                    ],
                ),
                SpecialModelPart::new(
                    SpecialPartPose::offset(0.0, -5.0, 0.0),
                    vec![SpecialModelCube::new(
                        [0.0, 27.0],
                        [-4.0, 0.0, -2.0],
                        [4.0, 5.0, 4.0],
                        tex,
                    )],
                    Vec::new(),
                ),
                SpecialModelPart::new(
                    SpecialPartPose::offset(0.0, -5.0, 0.0),
                    vec![SpecialModelCube::new(
                        [16.0, 27.0],
                        [0.0, 0.0, -2.0],
                        [4.0, 5.0, 4.0],
                        tex,
                    )],
                    Vec::new(),
                ),
            ],
        )]
    }

    fn get_texture(&self) -> String {
        let loc = el_core::resource_location::ResourceLocation::from_string(&self.texture);
        let path = loc
            .path
            .strip_prefix("textures/")
            .unwrap_or(&loc.path)
            .strip_suffix(".png")
            .unwrap_or_else(|| loc.path.strip_prefix("textures/").unwrap_or(&loc.path));
        format!("{}:{}", loc.namespace, path)
    }
}
