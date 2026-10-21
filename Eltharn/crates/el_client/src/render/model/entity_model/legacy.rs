#![allow(dead_code, unused_variables)]
use super::types::*;
use crate::render::baked_model::Vertex;
use el_math::Direction;
use glam::{Mat4, Vec3};
use std::collections::HashMap;
#[derive(Debug, Clone, Copy)]
pub struct CubeSpec {
    pub tex_offs: [f32; 2],
    pub origin: Vec3,
    pub dimensions: Vec3,
    pub grow: Vec3,
    pub mirror: bool,
}
impl CubeSpec {
    const fn new(
        tex_offs: [f32; 2],
        origin: Vec3,
        dimensions: Vec3,
        grow: Vec3,
        mirror: bool,
    ) -> Self {
        Self {
            tex_offs,
            origin,
            dimensions,
            grow,
            mirror,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct PartPose {
    pub offset: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}
impl PartPose {
    const ZERO: Self = Self {
        offset: Vec3::ZERO,
        rotation: Vec3::ZERO,
        scale: Vec3::ONE,
    };
    const fn offset(x: f32, y: f32, z: f32) -> Self {
        Self {
            offset: Vec3::new(x, y, z),
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
    const fn offset_and_rotation(
        x: f32,
        y: f32,
        z: f32,
        x_rot: f32,
        y_rot: f32,
        z_rot: f32,
    ) -> Self {
        Self {
            offset: Vec3::new(x, y, z),
            rotation: Vec3::new(x_rot, y_rot, z_rot),
            scale: Vec3::ONE,
        }
    }
    fn translated(self, x: f32, y: f32, z: f32) -> Self {
        Self {
            offset: self.offset + Vec3::new(x, y, z),
            ..self
        }
    }
    fn scaled(self, factor: f32) -> Self {
        Self {
            offset: self.offset * factor,
            scale: self.scale * factor,
            ..self
        }
    }
    fn matrix(self) -> Mat4 {
        Mat4::from_translation(self.offset)
            * Mat4::from_rotation_z(self.rotation.z)
            * Mat4::from_rotation_y(self.rotation.y)
            * Mat4::from_rotation_x(self.rotation.x)
            * Mat4::from_scale(self.scale)
    }
}
#[derive(Debug, Clone)]
pub struct PartSpec {
    pub pose: PartPose,
    pub cubes: Vec<CubeSpec>,
    pub children: Vec<PartSpec>,
}
impl PartSpec {
    fn new(pose: PartPose) -> Self {
        Self {
            pose,
            cubes: Vec::new(),
            children: Vec::new(),
        }
    }
    fn cube(mut self, cube: CubeSpec) -> Self {
        self.cubes.push(cube);
        self
    }
    fn child(mut self, child: PartSpec) -> Self {
        self.children.push(child);
        self
    }
}
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct NamedPartSpec {
    name: &'static str,
    pose: PartPose,
    cubes: Vec<CubeSpec>,
    children: Vec<NamedPartSpec>,
    visible: bool,
    skip_draw: bool,
}
#[allow(dead_code)]
impl NamedPartSpec {
    fn new(name: &'static str, pose: PartPose) -> Self {
        Self {
            name,
            pose,
            cubes: Vec::new(),
            children: Vec::new(),
            visible: true,
            skip_draw: false,
        }
    }
    fn cube(mut self, cube: CubeSpec) -> Self {
        self.cubes.push(cube);
        self
    }
    fn child(mut self, child: NamedPartSpec) -> Self {
        self.children.push(child);
        self
    }
    fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    fn skip_draw(mut self, skip_draw: bool) -> Self {
        self.skip_draw = skip_draw;
        self
    }
}
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JavaModelPartNode {
    name: &'static str,
    pose: PartPose,
    initial_pose: PartPose,
    cubes: Vec<CubeSpec>,
    children: Vec<usize>,
    visible: bool,
    skip_draw: bool,
}
#[allow(dead_code)]
impl JavaModelPartNode {
    fn load_pose(&mut self, pose: PartPose) {
        self.pose = pose;
    }
    fn reset_pose(&mut self) {
        self.load_pose(self.initial_pose);
    }
    fn set_pos(&mut self, x: f32, y: f32, z: f32) {
        self.pose.offset = Vec3::new(x, y, z);
    }
    fn set_rotation(&mut self, x_rot: f32, y_rot: f32, z_rot: f32) {
        self.pose.rotation = Vec3::new(x_rot, y_rot, z_rot);
    }
    fn offset_pos(&mut self, offset: Vec3) {
        self.pose.offset += offset;
    }
    fn offset_rotation(&mut self, offset: Vec3) {
        self.pose.rotation += offset;
    }
    fn offset_scale(&mut self, offset: Vec3) {
        self.pose.scale += offset;
    }
}
#[derive(Debug, Clone)]
pub(crate) struct JavaModelPartTree {
    nodes: Vec<JavaModelPartNode>,
    lookup: HashMap<&'static str, usize>,
}
#[allow(dead_code)]
impl JavaModelPartTree {
    fn from_root(root: NamedPartSpec) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            lookup: HashMap::new(),
        };
        tree.push_part(root);
        tree
    }
    fn push_part(&mut self, spec: NamedPartSpec) -> usize {
        let index = self.nodes.len();
        self.lookup.entry(spec.name).or_insert(index);
        self.nodes.push(JavaModelPartNode {
            name: spec.name,
            pose: spec.pose,
            initial_pose: spec.pose,
            cubes: spec.cubes,
            children: Vec::new(),
            visible: spec.visible,
            skip_draw: spec.skip_draw,
        });
        let children = spec
            .children
            .into_iter()
            .map(|child| self.push_part(child))
            .collect();
        self.nodes[index].children = children;
        index
    }
    fn part(&self, name: &str) -> Option<&JavaModelPartNode> {
        self.lookup
            .get(name)
            .and_then(|index| self.nodes.get(*index))
    }
    fn part_or_panic(&self, name: &str) -> &JavaModelPartNode {
        self.part(name)
            .unwrap_or_else(|| panic!("Can't find part {name}"))
    }
    fn part_mut(&mut self, name: &str) -> Option<&mut JavaModelPartNode> {
        let index = *self.lookup.get(name)?;
        self.nodes.get_mut(index)
    }
    fn part_mut_or_panic(&mut self, name: &str) -> &mut JavaModelPartNode {
        self.part_mut(name)
            .unwrap_or_else(|| panic!("Can't find part {name}"))
    }
    fn reset_pose(&mut self) {
        for node in &mut self.nodes {
            node.reset_pose();
        }
    }
    fn mesh(&self, texture: ModelTextureSize) -> (Vec<Vertex>, Vec<u32>) {
        let cube_count = self
            .nodes
            .iter()
            .filter(|node| node.visible && !node.skip_draw)
            .map(|node| node.cubes.len())
            .sum::<usize>();
        let mut vertices = Vec::with_capacity(cube_count * 24);
        let mut indices = Vec::with_capacity(cube_count * 36);
        if !self.nodes.is_empty() {
            collect_model_part_tree_vertices(
                self,
                0,
                Mat4::IDENTITY,
                texture,
                &mut vertices,
                &mut indices,
            );
        }
        (vertices, indices)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ModelTextureSize {
    pub width: f32,
    pub height: f32,
}
fn cube(tex_offs: [f32; 2], origin: [f32; 3], dimensions: [f32; 3]) -> CubeSpec {
    CubeSpec::new(
        tex_offs,
        Vec3::from(origin),
        Vec3::from(dimensions),
        Vec3::ZERO,
        false,
    )
}
fn cube_grown(tex_offs: [f32; 2], origin: [f32; 3], dimensions: [f32; 3], grow: f32) -> CubeSpec {
    CubeSpec::new(
        tex_offs,
        Vec3::from(origin),
        Vec3::from(dimensions),
        Vec3::splat(grow),
        false,
    )
}
fn cube_mirrored(tex_offs: [f32; 2], origin: [f32; 3], dimensions: [f32; 3]) -> CubeSpec {
    CubeSpec::new(
        tex_offs,
        Vec3::from(origin),
        Vec3::from(dimensions),
        Vec3::ZERO,
        true,
    )
}
pub fn build_mesh(root: PartSpec, texture: ModelTextureSize) -> (Vec<Vertex>, Vec<u32>) {
    let cube_count = part_cube_count(&root);
    let mut vertices = Vec::with_capacity(cube_count * 24);
    let mut indices = Vec::with_capacity(cube_count * 36);
    collect_part_vertices(&root, Mat4::IDENTITY, texture, &mut vertices, &mut indices);
    (vertices, indices)
}
fn part_cube_count(part: &PartSpec) -> usize {
    part.cubes.len() + part.children.iter().map(part_cube_count).sum::<usize>()
}
pub fn mesh_transform_scaling(mut root: PartSpec, factor: f32) -> PartSpec {
    let y_offset = 24.016 * (1.0 - factor);
    root.pose = root.pose.scaled(factor).translated(0.0, y_offset, 0.0);
    root
}
fn collect_part_vertices(
    part: &PartSpec,
    parent_matrix: Mat4,
    texture: ModelTextureSize,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
) {
    let part_matrix = parent_matrix * part.pose.matrix();
    for cube in &part.cubes {
        for polygon in cube_polygons(cube, texture) {
            let start = vertices.len() as u32;
            let normal = java_normal_to_world(
                part_matrix
                    .transform_vector3(polygon.normal)
                    .normalize_or_zero(),
            );
            for vertex in &polygon.vertices {
                let java_pos = part_matrix.transform_point3(vertex.pos);
                let world_pos = java_pos_to_world(java_pos);
                vertices.push(Vertex {
                    position: [world_pos.x, world_pos.y, world_pos.z],
                    tex_coords: vertex.uv,
                    normal: [normal.x, normal.y, normal.z],
                    color: [1.0, 1.0, 1.0, 1.0],
                    light: [1.0, 1.0],
                });
            }
            indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        }
    }
    for child in &part.children {
        collect_part_vertices(child, part_matrix, texture, vertices, indices);
    }
}
fn collect_model_part_tree_vertices(
    tree: &JavaModelPartTree,
    node_index: usize,
    parent_matrix: Mat4,
    texture: ModelTextureSize,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
) {
    let Some(node) = tree.nodes.get(node_index) else {
        return;
    };
    if !node.visible || (node.cubes.is_empty() && node.children.is_empty()) {
        return;
    }
    let part_matrix = parent_matrix * node.pose.matrix();
    if !node.skip_draw {
        for cube in &node.cubes {
            for polygon in cube_polygons(cube, texture) {
                let start = vertices.len() as u32;
                let normal = java_normal_to_world(
                    part_matrix
                        .transform_vector3(polygon.normal)
                        .normalize_or_zero(),
                );
                for vertex in &polygon.vertices {
                    let java_pos = part_matrix.transform_point3(vertex.pos);
                    let world_pos = java_pos_to_world(java_pos);
                    vertices.push(Vertex {
                        position: [world_pos.x, world_pos.y, world_pos.z],
                        tex_coords: vertex.uv,
                        normal: [normal.x, normal.y, normal.z],
                        color: [1.0, 1.0, 1.0, 1.0],
                        light: [1.0, 1.0],
                    });
                }
                indices.extend_from_slice(&[
                    start,
                    start + 1,
                    start + 2,
                    start,
                    start + 2,
                    start + 3,
                ]);
            }
        }
    }
    for child_index in &node.children {
        collect_model_part_tree_vertices(
            tree,
            *child_index,
            part_matrix,
            texture,
            vertices,
            indices,
        );
    }
}
fn java_pos_to_world(pos: Vec3) -> Vec3 {
    Vec3::new(pos.x, 24.0 - pos.y, -pos.z) / 16.0
}
fn java_normal_to_world(normal: Vec3) -> Vec3 {
    Vec3::new(normal.x, -normal.y, -normal.z).normalize_or_zero()
}
#[derive(Debug, Clone, Copy)]
struct ModelVertex {
    pos: Vec3,
    uv: [f32; 2],
}
#[derive(Debug, Clone, Copy)]
struct ModelPolygon {
    vertices: [ModelVertex; 4],
    normal: Vec3,
}
fn cube_polygons(cube: &CubeSpec, texture: ModelTextureSize) -> [ModelPolygon; 6] {
    let mut min_x = cube.origin.x - cube.grow.x;
    let min_y = cube.origin.y - cube.grow.y;
    let min_z = cube.origin.z - cube.grow.z;
    let mut max_x = cube.origin.x + cube.dimensions.x + cube.grow.x;
    let max_y = cube.origin.y + cube.dimensions.y + cube.grow.y;
    let max_z = cube.origin.z + cube.dimensions.z + cube.grow.z;
    if cube.mirror {
        std::mem::swap(&mut min_x, &mut max_x);
    }
    let t0 = Vec3::new(min_x, min_y, min_z);
    let t1 = Vec3::new(max_x, min_y, min_z);
    let t2 = Vec3::new(max_x, max_y, min_z);
    let t3 = Vec3::new(min_x, max_y, min_z);
    let l0 = Vec3::new(min_x, min_y, max_z);
    let l1 = Vec3::new(max_x, min_y, max_z);
    let l2 = Vec3::new(max_x, max_y, max_z);
    let l3 = Vec3::new(min_x, max_y, max_z);
    let width = cube.dimensions.x;
    let height = cube.dimensions.y;
    let depth = cube.dimensions.z;
    let u0 = cube.tex_offs[0];
    let u1 = cube.tex_offs[0] + depth;
    let u2 = cube.tex_offs[0] + depth + width;
    let u22 = cube.tex_offs[0] + depth + width + width;
    let u3 = cube.tex_offs[0] + depth + width + depth;
    let u4 = cube.tex_offs[0] + depth + width + depth + width;
    let v0 = cube.tex_offs[1];
    let v1 = cube.tex_offs[1] + depth;
    let v2 = cube.tex_offs[1] + depth + height;
    let polygon = |facing: Direction, positions: [Vec3; 4], uv_rect: [f32; 4]| {
        let [tu0, tv0, tu1, tv1] = uv_rect;
        let uv0 = [tu1 / texture.width, tv0 / texture.height];
        let uv1 = [tu0 / texture.width, tv0 / texture.height];
        let uv2 = [tu0 / texture.width, tv1 / texture.height];
        let uv3 = [tu1 / texture.width, tv1 / texture.height];
        let mut model_vertices = [
            ModelVertex {
                pos: positions[0],
                uv: uv0,
            },
            ModelVertex {
                pos: positions[1],
                uv: uv1,
            },
            ModelVertex {
                pos: positions[2],
                uv: uv2,
            },
            ModelVertex {
                pos: positions[3],
                uv: uv3,
            },
        ];
        let normal_facing = if cube.mirror {
            model_vertices.reverse();
            match facing {
                Direction::East => Direction::West,
                Direction::West => Direction::East,
                _ => facing,
            }
        } else {
            facing
        };
        ModelPolygon {
            vertices: model_vertices,
            normal: direction_normal(normal_facing),
        }
    };
    [
        polygon(Direction::Down, [l1, l0, t0, t1], [u1, v0, u2, v1]),
        polygon(Direction::Up, [t2, t3, l3, l2], [u2, v1, u22, v0]),
        polygon(Direction::West, [t0, l0, l3, t3], [u0, v1, u1, v2]),
        polygon(Direction::North, [t1, t0, t3, t2], [u1, v1, u2, v2]),
        polygon(Direction::East, [l1, t1, t2, l2], [u2, v1, u3, v2]),
        polygon(Direction::South, [l0, l1, l2, l3], [u3, v1, u4, v2]),
    ]
}
fn direction_normal(direction: Direction) -> Vec3 {
    match direction {
        Direction::Down => Vec3::NEG_Y,
        Direction::Up => Vec3::Y,
        Direction::North => Vec3::NEG_Z,
        Direction::South => Vec3::Z,
        Direction::West => Vec3::NEG_X,
        Direction::East => Vec3::X,
    }
}
#[derive(Debug, Clone, Copy)]
pub struct HumanoidAnimationPose {
    pub head_x_rot: f32,
    pub head_y_rot: f32,
    pub right_arm_x_rot: f32,
    pub left_arm_x_rot: f32,
    pub right_leg_x_rot: f32,
    pub left_leg_x_rot: f32,
}
pub fn humanoid_animation_pose(state: EntityModelAnimationState) -> HumanoidAnimationPose {
    let animation_pos = state.walk_animation_pos;
    let animation_speed = state.walk_animation_speed.min(1.0);
    let cycle = animation_pos * 0.6662;
    HumanoidAnimationPose {
        head_x_rot: state.x_rot_degrees.to_radians(),
        head_y_rot: state.y_rot_degrees.to_radians(),
        right_arm_x_rot: (cycle + std::f32::consts::PI).cos() * 2.0 * animation_speed * 0.5,
        left_arm_x_rot: cycle.cos() * 2.0 * animation_speed * 0.5,
        right_leg_x_rot: cycle.cos() * 1.4 * animation_speed,
        left_leg_x_rot: (cycle + std::f32::consts::PI).cos() * 1.4 * animation_speed,
    }
}
#[derive(Debug, Clone, Copy)]
pub struct QuadrupedAnimationPose {
    pub head_x_rot: f32,
    pub head_y_rot: f32,
    pub right_hind_leg_x_rot: f32,
    pub left_hind_leg_x_rot: f32,
    pub right_front_leg_x_rot: f32,
    pub left_front_leg_x_rot: f32,
}
pub fn quadruped_animation_pose(state: EntityModelAnimationState) -> QuadrupedAnimationPose {
    let animation_pos = state.walk_animation_pos;
    let animation_speed = state.walk_animation_speed.min(1.0);
    let cycle = animation_pos * 0.6662;
    QuadrupedAnimationPose {
        head_x_rot: state.x_rot_degrees.to_radians(),
        head_y_rot: state.y_rot_degrees.to_radians(),
        right_hind_leg_x_rot: cycle.cos() * 1.4 * animation_speed,
        left_hind_leg_x_rot: (cycle + std::f32::consts::PI).cos() * 1.4 * animation_speed,
        right_front_leg_x_rot: (cycle + std::f32::consts::PI).cos() * 1.4 * animation_speed,
        left_front_leg_x_rot: cycle.cos() * 1.4 * animation_speed,
    }
}
fn triangle_wave(index: f32, period: f32) -> f32 {
    ((index % period - period * 0.5).abs() - period * 0.25) / (period * 0.25)
}
pub fn build_humanoid_mesh(tex_width: f32, tex_height: f32) -> (Vec<Vertex>, Vec<u32>) {
    let root = PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset(0.0, 0.0, 0.0))
                .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
                    [32.0, 0.0],
                    [-4.0, -8.0, -4.0],
                    [8.0, 8.0, 8.0],
                    0.5,
                ))),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
            [40.0, 16.0],
            [-3.0, -2.0, -2.0],
            [4.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(5.0, 2.0, 0.0)).cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(PartSpec::new(PartPose::offset(-1.9, 12.0, 0.0)).cube(cube(
            [0.0, 16.0],
            [-2.0, 0.0, -2.0],
            [4.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(1.9, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: tex_width,
            height: tex_height,
        },
    )
}
pub fn build_humanoid_mesh_animated(
    tex_width: f32,
    tex_height: f32,
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let pose = humanoid_animation_pose(state);
    let root = PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0,
                0.0,
                0.0,
                pose.head_x_rot,
                pose.head_y_rot,
                0.0,
            ))
            .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
            .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
                [32.0, 0.0],
                [-4.0, -8.0, -4.0],
                [8.0, 8.0, 8.0],
                0.5,
            ))),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -5.0,
                2.0,
                0.0,
                pose.right_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube([40.0, 16.0], [-3.0, -2.0, -2.0], [4.0, 12.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                5.0,
                2.0,
                0.0,
                pose.left_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -1.9,
                12.0,
                0.0,
                pose.right_leg_x_rot,
                0.005,
                0.005,
            ))
            .cube(cube([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                1.9,
                12.0,
                0.0,
                pose.left_leg_x_rot,
                -0.005,
                -0.005,
            ))
            .cube(cube_mirrored(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: tex_width,
            height: tex_height,
        },
    )
}
pub fn build_piglin_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let ear_angle = std::f32::consts::PI / 6.0;
    let root = PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset(0.0, 0.0, 0.0))
                .cube(cube([0.0, 0.0], [-5.0, -8.0, -4.0], [10.0, 8.0, 8.0]))
                .cube(cube([31.0, 1.0], [-2.0, -4.0, -5.0], [4.0, 4.0, 1.0]))
                .cube(cube([2.0, 4.0], [2.0, -2.0, -5.0], [1.0, 2.0, 1.0]))
                .cube(cube([2.0, 0.0], [-3.0, -2.0, -5.0], [1.0, 2.0, 1.0]))
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        4.5, -6.0, 0.0, 0.0, 0.0, -ear_angle,
                    ))
                    .cube(cube(
                        [51.0, 6.0],
                        [0.0, 0.0, -2.0],
                        [1.0, 5.0, 4.0],
                    )),
                )
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        -4.5, -6.0, 0.0, 0.0, 0.0, ear_angle,
                    ))
                    .cube(cube(
                        [39.0, 6.0],
                        [-1.0, 0.0, -2.0],
                        [1.0, 5.0, 4.0],
                    )),
                ),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
            [40.0, 16.0],
            [-3.0, -2.0, -2.0],
            [4.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(5.0, 2.0, 0.0)).cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(PartSpec::new(PartPose::offset(-1.9, 12.0, 0.0)).cube(cube(
            [0.0, 16.0],
            [-2.0, 0.0, -2.0],
            [4.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(1.9, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 64.0,
        },
    )
}
pub fn build_piglin_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let pose = humanoid_animation_pose(state);
    let default_ear_angle = 30.0_f32.to_radians();
    let frequency = state.age_in_ticks * 0.1 + state.walk_animation_pos * 0.5;
    let amplitude = 0.08 + state.walk_animation_speed.min(1.0) * 0.4;
    let left_ear_z_rot = -default_ear_angle - (frequency * 1.2).cos() * amplitude;
    let right_ear_z_rot = default_ear_angle + frequency.cos() * amplitude;
    let root = PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0,
                0.0,
                0.0,
                pose.head_x_rot,
                pose.head_y_rot,
                0.0,
            ))
            .cube(cube([0.0, 0.0], [-5.0, -8.0, -4.0], [10.0, 8.0, 8.0]))
            .cube(cube([31.0, 1.0], [-2.0, -4.0, -5.0], [4.0, 4.0, 1.0]))
            .cube(cube([2.0, 4.0], [2.0, -2.0, -5.0], [1.0, 2.0, 1.0]))
            .cube(cube([2.0, 0.0], [-3.0, -2.0, -5.0], [1.0, 2.0, 1.0]))
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    4.5,
                    -6.0,
                    0.0,
                    0.0,
                    0.0,
                    left_ear_z_rot,
                ))
                .cube(cube([51.0, 6.0], [0.0, 0.0, -2.0], [1.0, 5.0, 4.0])),
            )
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    -4.5,
                    -6.0,
                    0.0,
                    0.0,
                    0.0,
                    right_ear_z_rot,
                ))
                .cube(cube([39.0, 6.0], [-1.0, 0.0, -2.0], [1.0, 5.0, 4.0])),
            ),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -5.0,
                2.0,
                0.0,
                pose.right_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube([40.0, 16.0], [-3.0, -2.0, -2.0], [4.0, 12.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                5.0,
                2.0,
                0.0,
                pose.left_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -1.9,
                12.0,
                0.0,
                pose.right_leg_x_rot,
                0.005,
                0.005,
            ))
            .cube(cube([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                1.9,
                12.0,
                0.0,
                pose.left_leg_x_rot,
                -0.005,
                -0.005,
            ))
            .cube(cube_mirrored(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 64.0,
        },
    )
}
fn skeleton_body_root() -> PartSpec {
    PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset(0.0, 0.0, 0.0))
                .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
                    [32.0, 0.0],
                    [-4.0, -8.0, -4.0],
                    [8.0, 8.0, 8.0],
                    0.5,
                ))),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
            [40.0, 16.0],
            [-1.0, -2.0, -1.0],
            [2.0, 12.0, 2.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(5.0, 2.0, 0.0)).cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
        .child(PartSpec::new(PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
            [0.0, 16.0],
            [-1.0, 0.0, -1.0],
            [2.0, 12.0, 2.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(2.0, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 16.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
}
fn skeleton_body_root_animated(state: EntityModelAnimationState) -> PartSpec {
    let pose = humanoid_animation_pose(state);
    PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0,
                0.0,
                0.0,
                pose.head_x_rot,
                pose.head_y_rot,
                0.0,
            ))
            .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
            .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
                [32.0, 0.0],
                [-4.0, -8.0, -4.0],
                [8.0, 8.0, 8.0],
                0.5,
            ))),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -5.0,
                2.0,
                0.0,
                pose.right_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube([40.0, 16.0], [-1.0, -2.0, -1.0], [2.0, 12.0, 2.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                5.0,
                2.0,
                0.0,
                pose.left_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -2.0,
                12.0,
                0.0,
                pose.right_leg_x_rot,
                0.005,
                0.005,
            ))
            .cube(cube([0.0, 16.0], [-1.0, 0.0, -1.0], [2.0, 12.0, 2.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                2.0,
                12.0,
                0.0,
                pose.left_leg_x_rot,
                -0.005,
                -0.005,
            ))
            .cube(cube_mirrored(
                [0.0, 16.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
}
pub fn build_skeleton_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let root = skeleton_body_root();
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_skeleton_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    build_mesh(
        skeleton_body_root_animated(state),
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_wither_skeleton_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let root = mesh_transform_scaling(skeleton_body_root(), 1.2);
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_wither_skeleton_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let root = mesh_transform_scaling(skeleton_body_root_animated(state), 1.2);
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_bogged_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let head = PartSpec::new(PartPose::offset(0.0, 0.0, 0.0))
        .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
        .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
            [32.0, 0.0],
            [-4.0, -8.0, -4.0],
            [8.0, 8.0, 8.0],
            0.5,
        )))
        .child(
            PartSpec::new(PartPose::ZERO)
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        3.0,
                        -8.0,
                        3.0,
                        0.0,
                        std::f32::consts::FRAC_PI_4,
                        0.0,
                    ))
                    .cube(cube(
                        [50.0, 16.0],
                        [-3.0, -3.0, 0.0],
                        [6.0, 4.0, 0.0],
                    )),
                )
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        3.0,
                        -8.0,
                        3.0,
                        0.0,
                        std::f32::consts::PI * 3.0 / 4.0,
                        0.0,
                    ))
                    .cube(cube(
                        [50.0, 16.0],
                        [-3.0, -3.0, 0.0],
                        [6.0, 4.0, 0.0],
                    )),
                )
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        -3.0,
                        -8.0,
                        -3.0,
                        0.0,
                        std::f32::consts::FRAC_PI_4,
                        0.0,
                    ))
                    .cube(cube(
                        [50.0, 22.0],
                        [-3.0, -3.0, 0.0],
                        [6.0, 4.0, 0.0],
                    )),
                )
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        -3.0,
                        -8.0,
                        -3.0,
                        0.0,
                        std::f32::consts::PI * 3.0 / 4.0,
                        0.0,
                    ))
                    .cube(cube(
                        [50.0, 22.0],
                        [-3.0, -3.0, 0.0],
                        [6.0, 4.0, 0.0],
                    )),
                )
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        -2.0,
                        -1.0,
                        4.0,
                        -std::f32::consts::FRAC_PI_2,
                        0.0,
                        std::f32::consts::FRAC_PI_4,
                    ))
                    .cube(cube(
                        [50.0, 28.0],
                        [-3.0, -4.0, 0.0],
                        [6.0, 4.0, 0.0],
                    )),
                )
                .child(
                    PartSpec::new(PartPose::offset_and_rotation(
                        -2.0,
                        -1.0,
                        4.0,
                        -std::f32::consts::FRAC_PI_2,
                        0.0,
                        std::f32::consts::PI * 3.0 / 4.0,
                    ))
                    .cube(cube(
                        [50.0, 28.0],
                        [-3.0, -4.0, 0.0],
                        [6.0, 4.0, 0.0],
                    )),
                ),
        );
    let root = PartSpec::new(PartPose::ZERO)
        .child(head)
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
            [40.0, 16.0],
            [-1.0, -2.0, -1.0],
            [2.0, 12.0, 2.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(5.0, 2.0, 0.0)).cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
        .child(PartSpec::new(PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
            [0.0, 16.0],
            [-1.0, 0.0, -1.0],
            [2.0, 12.0, 2.0],
        )))
        .child(
            PartSpec::new(PartPose::offset(2.0, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 16.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_bogged_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let pose = humanoid_animation_pose(state);
    let head = PartSpec::new(PartPose::offset_and_rotation(
        0.0,
        0.0,
        0.0,
        pose.head_x_rot,
        pose.head_y_rot,
        0.0,
    ))
    .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
    .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
        [32.0, 0.0],
        [-4.0, -8.0, -4.0],
        [8.0, 8.0, 8.0],
        0.5,
    )))
    .child(
        PartSpec::new(PartPose::ZERO)
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    3.0,
                    -8.0,
                    3.0,
                    0.0,
                    std::f32::consts::FRAC_PI_4,
                    0.0,
                ))
                .cube(cube([50.0, 16.0], [-3.0, -3.0, 0.0], [6.0, 4.0, 0.0])),
            )
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    3.0,
                    -8.0,
                    3.0,
                    0.0,
                    std::f32::consts::PI * 3.0 / 4.0,
                    0.0,
                ))
                .cube(cube([50.0, 16.0], [-3.0, -3.0, 0.0], [6.0, 4.0, 0.0])),
            )
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    -3.0,
                    -8.0,
                    -3.0,
                    0.0,
                    std::f32::consts::FRAC_PI_4,
                    0.0,
                ))
                .cube(cube([50.0, 22.0], [-3.0, -3.0, 0.0], [6.0, 4.0, 0.0])),
            )
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    -3.0,
                    -8.0,
                    -3.0,
                    0.0,
                    std::f32::consts::PI * 3.0 / 4.0,
                    0.0,
                ))
                .cube(cube([50.0, 22.0], [-3.0, -3.0, 0.0], [6.0, 4.0, 0.0])),
            )
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    -2.0,
                    -1.0,
                    4.0,
                    -std::f32::consts::FRAC_PI_2,
                    0.0,
                    std::f32::consts::FRAC_PI_4,
                ))
                .cube(cube([50.0, 28.0], [-3.0, -4.0, 0.0], [6.0, 4.0, 0.0])),
            )
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    -2.0,
                    -1.0,
                    4.0,
                    -std::f32::consts::FRAC_PI_2,
                    0.0,
                    std::f32::consts::PI * 3.0 / 4.0,
                ))
                .cube(cube([50.0, 28.0], [-3.0, -4.0, 0.0], [6.0, 4.0, 0.0])),
            ),
    );
    let root = PartSpec::new(PartPose::ZERO)
        .child(head)
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -5.0,
                2.0,
                0.0,
                pose.right_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube([40.0, 16.0], [-1.0, -2.0, -1.0], [2.0, 12.0, 2.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                5.0,
                2.0,
                0.0,
                pose.left_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube_mirrored(
                [40.0, 16.0],
                [-1.0, -2.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -2.0,
                12.0,
                0.0,
                pose.right_leg_x_rot,
                0.005,
                0.005,
            ))
            .cube(cube([0.0, 16.0], [-1.0, 0.0, -1.0], [2.0, 12.0, 2.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                2.0,
                12.0,
                0.0,
                pose.left_leg_x_rot,
                -0.005,
                -0.005,
            ))
            .cube(cube_mirrored(
                [0.0, 16.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_drowned_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let root = PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset(0.0, 0.0, 0.0))
                .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
                .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
                    [32.0, 0.0],
                    [-4.0, -8.0, -4.0],
                    [8.0, 8.0, 8.0],
                    0.5,
                ))),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(-5.0, 2.0, 0.0)).cube(cube(
            [40.0, 16.0],
            [-3.0, -2.0, -2.0],
            [4.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(5.0, 2.0, 0.0)).cube(cube(
            [32.0, 48.0],
            [-1.0, -2.0, -2.0],
            [4.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(-1.9, 12.0, 0.0)).cube(cube(
            [0.0, 16.0],
            [-2.0, 0.0, -2.0],
            [4.0, 12.0, 4.0],
        )))
        .child(PartSpec::new(PartPose::offset(1.9, 12.0, 0.0)).cube(cube(
            [16.0, 48.0],
            [-2.0, 0.0, -2.0],
            [4.0, 12.0, 4.0],
        )));
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 64.0,
        },
    )
}
pub fn build_drowned_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let pose = humanoid_animation_pose(state);
    let root = PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0,
                0.0,
                0.0,
                pose.head_x_rot,
                pose.head_y_rot,
                0.0,
            ))
            .cube(cube([0.0, 0.0], [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]))
            .child(PartSpec::new(PartPose::ZERO).cube(cube_grown(
                [32.0, 0.0],
                [-4.0, -8.0, -4.0],
                [8.0, 8.0, 8.0],
                0.5,
            ))),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
            [16.0, 16.0],
            [-4.0, 0.0, -2.0],
            [8.0, 12.0, 4.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -5.0,
                2.0,
                0.0,
                pose.right_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube([40.0, 16.0], [-3.0, -2.0, -2.0], [4.0, 12.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                5.0,
                2.0,
                0.0,
                pose.left_arm_x_rot,
                0.0,
                0.0,
            ))
            .cube(cube([32.0, 48.0], [-1.0, -2.0, -2.0], [4.0, 12.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -1.9,
                12.0,
                0.0,
                pose.right_leg_x_rot,
                0.005,
                0.005,
            ))
            .cube(cube([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                1.9,
                12.0,
                0.0,
                pose.left_leg_x_rot,
                -0.005,
                -0.005,
            ))
            .cube(cube([16.0, 48.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0])),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 64.0,
        },
    )
}
pub fn build_creeper_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_creeper_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_creeper_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_creeper_model_tree();
    apply_creeper_creeper_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_cow_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 4.0, -8.0))
                    .cube(cube([0.0, 0.0], [-4.0, -4.0, -6.0], [8.0, 8.0, 6.0]))
                    .cube(cube([1.0, 33.0], [-3.0, 1.0, -7.0], [6.0, 3.0, 1.0]))
                    .cube(cube([22.0, 0.0], [-5.0, -5.0, -5.0], [1.0, 3.0, 1.0]))
                    .cube(cube([22.0, 0.0], [4.0, -5.0, -5.0], [1.0, 3.0, 1.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 5.0, 2.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([18.0, 4.0], [-6.0, -10.0, -7.0], [12.0, 18.0, 10.0]))
                .cube(cube([52.0, 0.0], [-2.0, 2.0, -8.0], [4.0, 6.0, 1.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 12.0, 7.0)).cube(cube(
                    [0.0, 16.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 12.0, 7.0)).cube(
                    cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 12.0, -5.0))
                    .cube(cube([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 12.0, -5.0)).cube(
                    cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_cow_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cow_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_cow_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_cow_model_tree();
    apply_cow_cow_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_spider_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_spider_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_cave_spider_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cave_spider_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_spider_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_spider_model_tree();
    apply_spider_spider_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_cave_spider_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_cave_spider_model_tree();
    apply_cave_spider_spider_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_spider_mesh_unscaled() -> (Vec<Vertex>, Vec<u32>) {
    let right_leg = |x: f32, z: f32, y_rot: f32, z_rot: f32| {
        PartSpec::new(PartPose::offset_and_rotation(x, 15.0, z, 0.0, y_rot, z_rot)).cube(cube(
            [18.0, 0.0],
            [-15.0, -1.0, -1.0],
            [16.0, 2.0, 2.0],
        ))
    };
    let left_leg = |x: f32, z: f32, y_rot: f32, z_rot: f32| {
        PartSpec::new(PartPose::offset_and_rotation(x, 15.0, z, 0.0, y_rot, z_rot)).cube(
            cube_mirrored([18.0, 0.0], [-1.0, -1.0, -1.0], [16.0, 2.0, 2.0]),
        )
    };
    build_mesh(
        PartSpec::new(PartPose::ZERO)
            .child(PartSpec::new(PartPose::offset(0.0, 15.0, -3.0)).cube(cube(
                [32.0, 4.0],
                [-4.0, -4.0, -8.0],
                [8.0, 8.0, 8.0],
            )))
            .child(PartSpec::new(PartPose::offset(0.0, 15.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-3.0, -3.0, -3.0],
                [6.0, 6.0, 6.0],
            )))
            .child(PartSpec::new(PartPose::offset(0.0, 15.0, 9.0)).cube(cube(
                [0.0, 12.0],
                [-5.0, -4.0, -6.0],
                [10.0, 8.0, 12.0],
            )))
            .child(right_leg(
                -4.0,
                2.0,
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ))
            .child(left_leg(
                4.0,
                2.0,
                -std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ))
            .child(right_leg(
                -4.0,
                1.0,
                std::f32::consts::FRAC_PI_8,
                -0.58119464,
            ))
            .child(left_leg(4.0, 1.0, -std::f32::consts::FRAC_PI_8, 0.58119464))
            .child(right_leg(
                -4.0,
                0.0,
                -std::f32::consts::FRAC_PI_8,
                -0.58119464,
            ))
            .child(left_leg(4.0, 0.0, std::f32::consts::FRAC_PI_8, 0.58119464))
            .child(right_leg(
                -4.0,
                -1.0,
                -std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ))
            .child(left_leg(
                4.0,
                -1.0,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            )),
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_spider_mesh_animated_unscaled(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let animation_pos = state.walk_animation_pos * 0.6662;
    let animation_speed = state.walk_animation_speed.min(1.0);
    let swing_hind = -((animation_pos * 2.0 + 0.0).cos() * 0.4) * animation_speed;
    let swing_middle_hind =
        -((animation_pos * 2.0 + std::f32::consts::PI).cos() * 0.4) * animation_speed;
    let swing_middle_front =
        -((animation_pos * 2.0 + std::f32::consts::FRAC_PI_2).cos() * 0.4) * animation_speed;
    let swing_front =
        -((animation_pos * 2.0 + std::f32::consts::PI * 3.0 / 2.0).cos() * 0.4) * animation_speed;
    let step_hind = ((animation_pos + 0.0).sin() * 0.4).abs() * animation_speed;
    let step_middle_hind =
        ((animation_pos + std::f32::consts::PI).sin() * 0.4).abs() * animation_speed;
    let step_middle_front_hind =
        ((animation_pos + std::f32::consts::FRAC_PI_2).sin() * 0.4).abs() * animation_speed;
    let step_front =
        ((animation_pos + std::f32::consts::PI * 3.0 / 2.0).sin() * 0.4).abs() * animation_speed;
    let right_leg = |x: f32, z: f32, y_rot: f32, z_rot: f32| {
        PartSpec::new(PartPose::offset_and_rotation(x, 15.0, z, 0.0, y_rot, z_rot)).cube(cube(
            [18.0, 0.0],
            [-15.0, -1.0, -1.0],
            [16.0, 2.0, 2.0],
        ))
    };
    let left_leg = |x: f32, z: f32, y_rot: f32, z_rot: f32| {
        PartSpec::new(PartPose::offset_and_rotation(x, 15.0, z, 0.0, y_rot, z_rot)).cube(
            cube_mirrored([18.0, 0.0], [-1.0, -1.0, -1.0], [16.0, 2.0, 2.0]),
        )
    };
    build_mesh(
        PartSpec::new(PartPose::ZERO)
            .child(
                PartSpec::new(PartPose::offset_and_rotation(
                    0.0,
                    15.0,
                    -3.0,
                    state.x_rot_degrees.to_radians(),
                    state.y_rot_degrees.to_radians(),
                    0.0,
                ))
                .cube(cube([32.0, 4.0], [-4.0, -4.0, -8.0], [8.0, 8.0, 8.0])),
            )
            .child(PartSpec::new(PartPose::offset(0.0, 15.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-3.0, -3.0, -3.0],
                [6.0, 6.0, 6.0],
            )))
            .child(PartSpec::new(PartPose::offset(0.0, 15.0, 9.0)).cube(cube(
                [0.0, 12.0],
                [-5.0, -4.0, -6.0],
                [10.0, 8.0, 12.0],
            )))
            .child(right_leg(
                -4.0,
                2.0,
                std::f32::consts::FRAC_PI_4 + swing_hind,
                -std::f32::consts::FRAC_PI_4 + step_hind,
            ))
            .child(left_leg(
                4.0,
                2.0,
                -std::f32::consts::FRAC_PI_4 - swing_hind,
                std::f32::consts::FRAC_PI_4 - step_hind,
            ))
            .child(right_leg(
                -4.0,
                1.0,
                std::f32::consts::FRAC_PI_8 + swing_middle_hind,
                -0.58119464 + step_middle_hind,
            ))
            .child(left_leg(
                4.0,
                1.0,
                -std::f32::consts::FRAC_PI_8 - swing_middle_hind,
                0.58119464 - step_middle_hind,
            ))
            .child(right_leg(
                -4.0,
                0.0,
                -std::f32::consts::FRAC_PI_8 + swing_middle_front,
                -0.58119464 + step_middle_front_hind,
            ))
            .child(left_leg(
                4.0,
                0.0,
                std::f32::consts::FRAC_PI_8 - swing_middle_front,
                0.58119464 - step_middle_front_hind,
            ))
            .child(right_leg(
                -4.0,
                -1.0,
                -std::f32::consts::FRAC_PI_4 + swing_front,
                -std::f32::consts::FRAC_PI_4 + step_front,
            ))
            .child(left_leg(
                4.0,
                -1.0,
                std::f32::consts::FRAC_PI_4 - swing_front,
                std::f32::consts::FRAC_PI_4 - step_front,
            )),
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
fn scale_vertices_like_java_mesh_transformer(
    mut vertices: Vec<Vertex>,
    factor: f32,
) -> Vec<Vertex> {
    let y_offset = (24.016 - 24.0) / 16.0 * (1.0 - factor);
    for vertex in &mut vertices {
        vertex.position[0] *= factor;
        vertex.position[1] = vertex.position[1] * factor - y_offset;
        vertex.position[2] *= factor;
    }
    vertices
}
fn adult_chicken_texture_size() -> ModelTextureSize {
    ModelTextureSize {
        width: 64.0,
        height: 32.0,
    }
}
pub fn build_adult_chicken_model_tree() -> JavaModelPartTree {
    JavaModelPartTree::from_root(
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -4.0))
                    .cube(cube([0.0, 0.0], [-2.0, -6.0, -2.0], [4.0, 6.0, 3.0]))
                    .child(NamedPartSpec::new("beak", PartPose::ZERO).cube(cube(
                        [14.0, 0.0],
                        [-2.0, -4.0, -4.0],
                        [4.0, 2.0, 2.0],
                    )))
                    .child(NamedPartSpec::new("red_thing", PartPose::ZERO).cube(cube(
                        [14.0, 4.0],
                        [-1.0, -2.0, -3.0],
                        [2.0, 2.0, 2.0],
                    ))),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(
                        0.0,
                        16.0,
                        0.0,
                        std::f32::consts::FRAC_PI_2,
                        0.0,
                        0.0,
                    ),
                )
                .cube(cube([0.0, 9.0], [-3.0, -4.0, -3.0], [6.0, 8.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 19.0, 1.0)).cube(cube(
                    [26.0, 0.0],
                    [-1.0, 0.0, -3.0],
                    [3.0, 5.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(1.0, 19.0, 1.0)).cube(cube(
                    [26.0, 0.0],
                    [-1.0, 0.0, -3.0],
                    [3.0, 5.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_wing", PartPose::offset(-4.0, 13.0, 0.0)).cube(cube(
                    [24.0, 13.0],
                    [0.0, 0.0, -3.0],
                    [1.0, 4.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_wing", PartPose::offset(4.0, 13.0, 0.0)).cube(cube(
                    [24.0, 13.0],
                    [-1.0, 0.0, -3.0],
                    [1.0, 4.0, 6.0],
                )),
            ),
    )
}
fn setup_chicken_model_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    tree.reset_pose();
    let flap_angle = (state.flap.sin() + 1.0) * state.flap_speed;
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    tree.part_mut_or_panic("right_leg").pose.rotation.x =
        (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_leg").pose.rotation.x =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_wing").pose.rotation.z = flap_angle;
    tree.part_mut_or_panic("left_wing").pose.rotation.z = -flap_angle;
}
fn setup_adult_chicken_model_anim(tree: &mut JavaModelPartTree, state: EntityModelAnimationState) {
    setup_chicken_model_anim(tree, state);
    let head = tree.part_mut_or_panic("head");
    head.pose.rotation.x = state.x_rot_degrees.to_radians();
    head.pose.rotation.y = state.y_rot_degrees.to_radians();
}
pub fn build_chicken_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -4.0))
                .cube(cube([0.0, 0.0], [-2.0, -6.0, -2.0], [4.0, 6.0, 3.0]))
                .child(NamedPartSpec::new("beak", PartPose::ZERO).cube(cube(
                    [14.0, 0.0],
                    [-2.0, -4.0, -4.0],
                    [4.0, 2.0, 2.0],
                )))
                .child(NamedPartSpec::new("red_thing", PartPose::ZERO).cube(cube(
                    [14.0, 4.0],
                    [-1.0, -2.0, -3.0],
                    [2.0, 2.0, 2.0],
                ))),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 16.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 9.0], [-3.0, -4.0, -3.0], [6.0, 8.0, 6.0])),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 19.0, 1.0)).cube(cube(
                [26.0, 0.0],
                [-1.0, 0.0, -3.0],
                [3.0, 5.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(1.0, 19.0, 1.0)).cube(cube(
                [26.0, 0.0],
                [-1.0, 0.0, -3.0],
                [3.0, 5.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_wing", PartPose::offset(-4.0, 13.0, 0.0)).cube(cube(
                [24.0, 13.0],
                [0.0, 0.0, -3.0],
                [1.0, 4.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_wing", PartPose::offset(4.0, 13.0, 0.0)).cube(cube(
                [24.0, 13.0],
                [-1.0, 0.0, -3.0],
                [1.0, 4.0, 6.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_chicken_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_chicken_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_chicken_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_chicken_model_tree();
    apply_chicken_adult_chicken_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_bat_model_tree() -> JavaModelPartTree {
    let body = NamedPartSpec::new("body", PartPose::offset(0.0, 17.0, 0.0))
        .cube(cube([0.0, 0.0], [-1.5, 0.0, -1.0], [3.0, 5.0, 2.0]))
        .child(
            NamedPartSpec::new("right_wing", PartPose::offset(-1.5, 0.0, 0.0))
                .cube(cube([12.0, 0.0], [-2.0, -2.0, 0.0], [2.0, 7.0, 0.0]))
                .child(
                    NamedPartSpec::new("right_wing_tip", PartPose::offset(-2.0, 0.0, 0.0))
                        .cube(cube([16.0, 0.0], [-6.0, -2.0, 0.0], [6.0, 8.0, 0.0])),
                ),
        )
        .child(
            NamedPartSpec::new("left_wing", PartPose::offset(1.5, 0.0, 0.0))
                .cube(cube([12.0, 7.0], [0.0, -2.0, 0.0], [2.0, 7.0, 0.0]))
                .child(
                    NamedPartSpec::new("left_wing_tip", PartPose::offset(2.0, 0.0, 0.0))
                        .cube(cube([16.0, 8.0], [0.0, -2.0, 0.0], [6.0, 8.0, 0.0])),
                ),
        )
        .child(
            NamedPartSpec::new("feet", PartPose::offset(0.0, 5.0, 0.0)).cube(cube(
                [16.0, 16.0],
                [-1.5, 0.0, 0.0],
                [3.0, 2.0, 0.0],
            )),
        );
    let head = NamedPartSpec::new("head", PartPose::offset(0.0, 17.0, 0.0))
        .cube(cube([0.0, 7.0], [-2.0, -3.0, -1.0], [4.0, 3.0, 2.0]))
        .child(
            NamedPartSpec::new("right_ear", PartPose::offset(-1.5, -2.0, 0.0)).cube(cube(
                [1.0, 15.0],
                [-2.5, -4.0, 0.0],
                [3.0, 5.0, 0.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_ear", PartPose::offset(1.1, -3.0, 0.0)).cube(cube(
                [8.0, 15.0],
                [-0.1, -3.0, 0.0],
                [3.0, 5.0, 0.0],
            )),
        );
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(body)
        .child(head);
    JavaModelPartTree::from_root(root)
}
pub fn build_bat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_bat_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn apply_bat_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if state.is_resting {
        if let Some(head) = tree.part_mut("head") {
            head.pose.rotation.y = state.y_rot_degrees.to_radians();
        }
    } else {
        let flap = (state.age_in_ticks * 0.5).sin();
        if let Some(right_wing) = tree.part_mut("right_wing") {
            right_wing.pose.rotation.y = flap * 0.8;
        }
        if let Some(left_wing) = tree.part_mut("left_wing") {
            left_wing.pose.rotation.y = -flap * 0.8;
        }
        if let Some(right_wing_tip) = tree.part_mut("right_wing_tip") {
            right_wing_tip.pose.rotation.y = flap * 0.4;
        }
        if let Some(left_wing_tip) = tree.part_mut("left_wing_tip") {
            left_wing_tip.pose.rotation.y = -flap * 0.4;
        }
    }
}
pub fn build_bat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_bat_model_tree();
    apply_bat_model_setup_anim_ir(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
fn boat_common_root() -> PartSpec {
    let pi = std::f32::consts::PI;
    PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0,
                3.0,
                1.0,
                std::f32::consts::FRAC_PI_2,
                0.0,
                0.0,
            ))
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -15.0,
                4.0,
                4.0,
                0.0,
                pi * 1.5,
                0.0,
            ))
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                15.0,
                4.0,
                0.0,
                0.0,
                std::f32::consts::FRAC_PI_2,
                0.0,
            ))
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, pi, 0.0)).cube(cube(
                [0.0, 35.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
            [0.0, 43.0],
            [-14.0, -7.0, -1.0],
            [28.0, 6.0, 2.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                3.0,
                -5.0,
                9.0,
                0.0,
                0.0,
                pi / 16.0,
            ))
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                3.0,
                -5.0,
                -9.0,
                0.0,
                pi,
                pi / 16.0,
            ))
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
}
pub fn build_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_mesh(
        boat_common_root(),
        ModelTextureSize {
            width: 128.0,
            height: 64.0,
        },
    )
}
pub fn build_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let root = boat_common_root()
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -2.0,
                -5.0,
                -6.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                0.0,
            ))
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -2.0,
                -9.0,
                -6.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                0.0,
            ))
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -1.0,
                -6.0,
                -1.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                0.0,
            ))
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 128.0,
            height: 128.0,
        },
    )
}
fn raft_common_root() -> PartSpec {
    let pi = std::f32::consts::PI;
    PartSpec::new(PartPose::ZERO)
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0, -2.1, 1.0, 1.5708, 0.0, 0.0,
            ))
            .cube(cube([0.0, 0.0], [-14.0, -11.0, -4.0], [28.0, 20.0, 4.0]))
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -8.0], [28.0, 16.0, 4.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                3.0,
                -4.0,
                9.0,
                0.0,
                0.0,
                pi / 16.0,
            ))
            .cube(cube([0.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([0.0, 24.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                3.0,
                -4.0,
                -9.0,
                0.0,
                pi,
                pi / 16.0,
            ))
            .cube(cube([40.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([40.0, 24.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
}
pub fn build_raft_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_mesh(
        raft_common_root(),
        ModelTextureSize {
            width: 128.0,
            height: 64.0,
        },
    )
}
pub fn build_chest_raft_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let root = raft_common_root()
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -2.0,
                -10.1,
                -6.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                0.0,
            ))
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -2.0,
                -14.1,
                -6.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                0.0,
            ))
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -1.0,
                -11.1,
                -1.0,
                0.0,
                -std::f32::consts::FRAC_PI_2,
                0.0,
            ))
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 128.0,
            height: 128.0,
        },
    )
}
pub fn build_minecart_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_leash_knot_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("knot", PartPose::ZERO).cube(cube(
            [0.0, 0.0],
            [-3.0, -8.0, -3.0],
            [6.0, 8.0, 6.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_leash_knot_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_leash_knot_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn build_llama_spit_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("main", PartPose::ZERO)
            .cube(cube([0.0, 0.0], [-4.0, 0.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, -4.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 0.0, -4.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 2.0, 0.0], [2.0, 2.0, 2.0]))
            .cube(cube([0.0, 0.0], [0.0, 0.0, 2.0], [2.0, 2.0, 2.0])),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_llama_spit_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_llama_spit_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_trident_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("pole", PartPose::ZERO)
            .cube(cube([0.0, 6.0], [-0.5, 2.0, -0.5], [1.0, 25.0, 1.0]))
            .child(NamedPartSpec::new("base", PartPose::ZERO).cube(cube(
                [4.0, 0.0],
                [-1.5, 0.0, -0.5],
                [3.0, 2.0, 1.0],
            )))
            .child(NamedPartSpec::new("left_spike", PartPose::ZERO).cube(cube(
                [4.0, 3.0],
                [-2.5, -3.0, -0.5],
                [1.0, 4.0, 1.0],
            )))
            .child(
                NamedPartSpec::new("middle_spike", PartPose::ZERO).cube(cube(
                    [0.0, 0.0],
                    [-0.5, -4.0, -0.5],
                    [1.0, 4.0, 1.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_spike", PartPose::ZERO).cube(cube_mirrored(
                    [4.0, 3.0],
                    [1.5, -3.0, -0.5],
                    [1.0, 4.0, 1.0],
                )),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_trident_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_trident_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn build_slime_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(NamedPartSpec::new("cube", PartPose::ZERO).cube(cube(
            [0.0, 16.0],
            [-3.0, 17.0, -3.0],
            [6.0, 6.0, 6.0],
        )))
        .child(NamedPartSpec::new("right_eye", PartPose::ZERO).cube(cube(
            [32.0, 0.0],
            [-3.25, 18.0, -3.5],
            [2.0, 2.0, 2.0],
        )))
        .child(NamedPartSpec::new("left_eye", PartPose::ZERO).cube(cube(
            [32.0, 4.0],
            [1.25, 18.0, -3.5],
            [2.0, 2.0, 2.0],
        )))
        .child(NamedPartSpec::new("mouth", PartPose::ZERO).cube(cube(
            [32.0, 8.0],
            [0.0, 21.0, -3.5],
            [1.0, 1.0, 1.0],
        )));
    JavaModelPartTree::from_root(root)
}
pub fn build_slime_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_slime_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_slime_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_slime_model_tree();
    apply_slime_slime_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_sulfur_cube_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("cube", PartPose::ZERO).cube(cube(
            [0.0, 0.0],
            [-9.0, -9.0, -9.0],
            [18.0, 18.0, 18.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_sulfur_cube_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_sulfur_cube_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_armadillo_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 21.0, 4.0))
                    .cube(cube_grown(
                        [0.0, 20.0],
                        [-4.0, -7.0, -10.0],
                        [8.0, 8.0, 12.0],
                        0.3,
                    ))
                    .cube(cube([0.0, 40.0], [-4.0, -7.0, -10.0], [8.0, 8.0, 12.0]))
                    .child(
                        NamedPartSpec::new(
                            "tail",
                            PartPose::offset_and_rotation(0.0, -3.0, 1.0, 0.5061, 0.0, 0.0),
                        )
                        .cube(cube(
                            [44.0, 53.0],
                            [-0.5, -0.0865, 0.0933],
                            [1.0, 6.0, 1.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, -2.0, -11.0))
                            .child(
                                NamedPartSpec::new(
                                    "head_cube",
                                    PartPose::offset_and_rotation(0.0, 0.0, 0.0, -0.3927, 0.0, 0.0),
                                )
                                .cube(cube(
                                    [43.0, 15.0],
                                    [-1.5, -1.0, -1.0],
                                    [3.0, 5.0, 2.0],
                                )),
                            )
                            .child(
                                NamedPartSpec::new("right_ear", PartPose::offset(-1.0, -1.0, 0.0))
                                    .child(
                                        NamedPartSpec::new(
                                            "right_ear_cube",
                                            PartPose::offset_and_rotation(
                                                -0.5, 0.0, -0.6, 0.1886, -0.3864, -0.0718,
                                            ),
                                        )
                                        .cube(cube(
                                            [43.0, 10.0],
                                            [-2.0, -3.0, 0.0],
                                            [2.0, 5.0, 0.0],
                                        )),
                                    ),
                            )
                            .child(
                                NamedPartSpec::new("left_ear", PartPose::offset(1.0, -2.0, 0.0))
                                    .child(
                                        NamedPartSpec::new(
                                            "left_ear_cube",
                                            PartPose::offset_and_rotation(
                                                0.5, 1.0, -0.6, 0.1886, 0.3864, 0.0718,
                                            ),
                                        )
                                        .cube(cube(
                                            [47.0, 10.0],
                                            [0.0, -3.0, 0.0],
                                            [2.0, 5.0, 0.0],
                                        )),
                                    ),
                            ),
                    ),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-2.0, 21.0, 4.0)).cube(cube(
                    [51.0, 31.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 3.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(2.0, 21.0, 4.0)).cube(cube(
                    [42.0, 31.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 3.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-2.0, 21.0, -4.0))
                    .cube(cube([51.0, 43.0], [-1.0, 0.0, -1.0], [2.0, 3.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(2.0, 21.0, -4.0)).cube(cube(
                    [42.0, 43.0],
                    [-1.0, 0.0, -1.0],
                    [2.0, 3.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("cube", PartPose::offset(0.0, 24.0, 0.0))
                    .visible(false)
                    .cube(cube([0.0, 0.0], [-5.0, -10.0, -6.0], [10.0, 10.0, 10.0])),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_armadillo_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_armadillo_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_armadillo_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if state.is_hiding_in_shell {
        if let Some(body) = tree.part_mut("body") {
            body.skip_draw = true;
        }
        if let Some(left_hind_leg) = tree.part_mut("left_hind_leg") {
            left_hind_leg.visible = false;
        }
        if let Some(right_hind_leg) = tree.part_mut("right_hind_leg") {
            right_hind_leg.visible = false;
        }
        if let Some(left_front_leg) = tree.part_mut("left_front_leg") {
            left_front_leg.visible = false;
        }
        if let Some(right_front_leg) = tree.part_mut("right_front_leg") {
            right_front_leg.visible = false;
        }
        if let Some(tail) = tree.part_mut("tail") {
            tail.visible = false;
        }
        if let Some(cube) = tree.part_mut("cube") {
            cube.visible = true;
        }
    } else {
        if let Some(body) = tree.part_mut("body") {
            body.skip_draw = false;
        }
        if let Some(left_hind_leg) = tree.part_mut("left_hind_leg") {
            left_hind_leg.visible = true;
        }
        if let Some(right_hind_leg) = tree.part_mut("right_hind_leg") {
            right_hind_leg.visible = true;
        }
        if let Some(left_front_leg) = tree.part_mut("left_front_leg") {
            left_front_leg.visible = true;
        }
        if let Some(right_front_leg) = tree.part_mut("right_front_leg") {
            right_front_leg.visible = true;
        }
        if let Some(tail) = tree.part_mut("tail") {
            tail.visible = true;
        }
        if let Some(cube) = tree.part_mut("cube") {
            cube.visible = false;
        }
        if let Some(head) = tree.part_mut("head") {
            head.pose.rotation.x = state.x_rot_degrees.clamp(-22.5, 25.0).to_radians();
            head.pose.rotation.y = state.y_rot_degrees.clamp(-32.5, 32.5).to_radians();
        }
        let speed = state.walk_animation_speed.min(1.0);
        let walk = state.walk_animation_pos;
        let leg_rot = (walk * 0.6662).cos() * 1.4 * speed;
        if let Some(leg) = tree.part_mut("right_hind_leg") {
            leg.pose.rotation.x = leg_rot;
        }
        if let Some(leg) = tree.part_mut("left_hind_leg") {
            leg.pose.rotation.x = -leg_rot;
        }
        if let Some(leg) = tree.part_mut("right_front_leg") {
            leg.pose.rotation.x = -leg_rot;
        }
        if let Some(leg) = tree.part_mut("left_front_leg") {
            leg.pose.rotation.x = leg_rot;
        }
    }
}
pub fn build_armadillo_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_armadillo_model_tree();
    apply_armadillo_model_setup_anim_ir(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_rabbit_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 23.0, 4.0, -0.3927, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-4.0, -6.0, -9.0], [8.0, 6.0, 10.0]))
            .child(
                NamedPartSpec::new("tail", PartPose::offset(0.0, -4.9916, 0.0125)).cube(cube(
                    [20.0, 16.0],
                    [-2.0, -3.0084, -1.0125],
                    [4.0, 4.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "head",
                    PartPose::offset_and_rotation(0.0, -5.2929, -8.1213, 0.3927, 0.0, 0.0),
                )
                .cube(cube([0.0, 16.0], [-2.5, -3.0, -4.0], [5.0, 5.0, 5.0]))
                .child(
                    NamedPartSpec::new("left_ear", PartPose::offset(1.5, -3.7071, -0.8787))
                        .cube(cube([32.0, 0.0], [-1.0, -4.2929, -0.1213], [2.0, 5.0, 1.0])),
                )
                .child(
                    NamedPartSpec::new("right_ear", PartPose::offset(-1.5, -3.7071, -0.8787))
                        .cube(cube([26.0, 0.0], [-1.0, -4.2929, -0.1213], [2.0, 5.0, 1.0])),
                ),
            )
            .child(
                NamedPartSpec::new("frontlegs", PartPose::offset(0.0, -1.5349, -6.3108))
                    .child(
                        NamedPartSpec::new(
                            "right_front_leg",
                            PartPose::offset_and_rotation(-2.0, 1.9239, 0.3827, 0.3927, 0.0, 0.0),
                        )
                        .cube(cube(
                            [36.0, 18.0],
                            [-0.9, -1.0, -0.9],
                            [2.0, 4.0, 2.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new(
                            "left_front_leg",
                            PartPose::offset_and_rotation(2.0, 1.9239, 0.4827, 0.3927, 0.0, 0.0),
                        )
                        .cube(cube(
                            [44.0, 18.0],
                            [-1.0, -1.0, -1.0],
                            [2.0, 4.0, 2.0],
                        )),
                    ),
            ),
        )
        .child(
            NamedPartSpec::new("backlegs", PartPose::offset(0.0, 23.0, 4.0))
                .child(
                    NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 0.5, 0.0)).child(
                        NamedPartSpec::new(
                            "right_haunch",
                            PartPose::offset_and_rotation(0.0, -0.5, 0.0, 0.0, 0.3927, 0.0),
                        )
                        .cube(cube(
                            [20.0, 24.0],
                            [-1.0, 0.0, -5.0],
                            [2.0, 1.0, 6.0],
                        )),
                    ),
                )
                .child(
                    NamedPartSpec::new("left_hind_leg", PartPose::offset(3.0, 0.5, 0.0)).child(
                        NamedPartSpec::new(
                            "left_haunch",
                            PartPose::offset_and_rotation(0.0, -0.5, 0.0, 0.0, -0.3927, 0.0),
                        )
                        .cube(cube(
                            [36.0, 24.0],
                            [-1.0, 0.0, -5.0],
                            [2.0, 1.0, 6.0],
                        )),
                    ),
                ),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_rabbit_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_rabbit_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_rabbit_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    }
}
pub fn build_rabbit_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_rabbit_model_tree();
    tree.reset_pose();
    apply_rabbit_model_setup_anim_ir(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_tadpole_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_tadpole_model_tree().mesh(ModelTextureSize {
        width: 16.0,
        height: 16.0,
    })
}
pub fn build_tadpole_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_tadpole_model_tree();
    apply_tadpole_tadpole_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 16.0,
        height: 16.0,
    })
}
pub fn build_magma_cube_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_magma_cube_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_magma_cube_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_magma_cube_model_tree();
    apply_magma_cube_magma_cube_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_snow_golem_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_snow_golem_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_snow_golem_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_snow_golem_model_tree();
    apply_snow_golem_snow_golem_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_allay_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_allay_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn build_pufferfish_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let root = PartSpec::new(PartPose::ZERO)
        .child(PartSpec::new(PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
            [0.0, 0.0],
            [-4.0, -8.0, -4.0],
            [8.0, 8.0, 8.0],
        )))
        .child(PartSpec::new(PartPose::offset(-4.0, 15.0, -2.0)).cube(cube(
            [24.0, 0.0],
            [-2.0, 0.0, -1.0],
            [2.0, 1.0, 2.0],
        )))
        .child(PartSpec::new(PartPose::offset(4.0, 15.0, -2.0)).cube(cube(
            [24.0, 3.0],
            [0.0, 0.0, -1.0],
            [2.0, 1.0, 2.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0, 14.0, -4.0, 0.785398, 0.0, 0.0,
            ))
            .cube(cube([15.0, 17.0], [-4.0, -1.0, 0.0], [8.0, 1.0, 0.0])),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 14.0, 0.0)).cube(cube(
            [14.0, 16.0],
            [-4.0, -1.0, 0.0],
            [8.0, 1.0, 1.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0, 14.0, 4.0, -0.785398, 0.0, 0.0,
            ))
            .cube(cube([23.0, 18.0], [-4.0, -1.0, 0.0], [8.0, 1.0, 0.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -4.0, 22.0, -4.0, 0.0, -0.785398, 0.0,
            ))
            .cube(cube([5.0, 17.0], [-1.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                4.0, 22.0, -4.0, 0.0, 0.785398, 0.0,
            ))
            .cube(cube([1.0, 17.0], [0.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0, 22.0, -4.0, -0.785398, 0.0, 0.0,
            ))
            .cube(cube([15.0, 20.0], [-4.0, 0.0, 0.0], [8.0, 1.0, 0.0])),
        )
        .child(PartSpec::new(PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
            [15.0, 20.0],
            [-4.0, 0.0, 0.0],
            [8.0, 1.0, 0.0],
        )))
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                0.0, 22.0, 4.0, 0.785398, 0.0, 0.0,
            ))
            .cube(cube([15.0, 20.0], [-4.0, 0.0, 0.0], [8.0, 1.0, 0.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                -4.0, 22.0, 4.0, 0.0, 0.785398, 0.0,
            ))
            .cube(cube([9.0, 17.0], [-1.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
        )
        .child(
            PartSpec::new(PartPose::offset_and_rotation(
                4.0, 22.0, 4.0, 0.0, -0.785398, 0.0,
            ))
            .cube(cube([9.0, 17.0], [0.0, -8.0, 0.0], [1.0, 8.0, 0.0])),
        );
    build_mesh(
        root,
        ModelTextureSize {
            width: 32.0,
            height: 32.0,
        },
    )
}
pub fn build_squid_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut root =
        PartSpec::new(PartPose::ZERO).child(PartSpec::new(PartPose::offset(0.0, 8.0, 0.0)).cube(
            cube_grown([0.0, 0.0], [-6.0, -8.0, -6.0], [12.0, 16.0, 12.0], 0.02),
        ));
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::PI * 2.0 / 8.0;
        let x = angle.cos() * 5.0;
        let z = angle.sin() * 5.0;
        let y_rot = i as f32 * std::f32::consts::PI * -2.0 / 8.0 + std::f32::consts::FRAC_PI_2;
        root =
            root.child(
                PartSpec::new(PartPose::offset_and_rotation(x, 15.0, z, 0.0, y_rot, 0.0))
                    .cube(cube([48.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 18.0, 2.0])),
            );
    }
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_squid_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut root =
        PartSpec::new(PartPose::ZERO).child(PartSpec::new(PartPose::offset(0.0, 8.0, 0.0)).cube(
            cube_grown([0.0, 0.0], [-6.0, -8.0, -6.0], [12.0, 16.0, 12.0], 0.02),
        ));
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::PI * 2.0 / 8.0;
        let x = angle.cos() * 5.0;
        let z = angle.sin() * 5.0;
        let y_rot = i as f32 * std::f32::consts::PI * -2.0 / 8.0 + std::f32::consts::FRAC_PI_2;
        root = root.child(
            PartSpec::new(PartPose::offset_and_rotation(
                x,
                15.0,
                z,
                state.tentacle_angle,
                y_rot,
                0.0,
            ))
            .cube(cube([48.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 18.0, 2.0])),
        );
    }
    build_mesh(
        root,
        ModelTextureSize {
            width: 64.0,
            height: 32.0,
        },
    )
}
pub fn build_iron_golem_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_iron_golem_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_iron_golem_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_iron_golem_model_tree();
    apply_iron_golem_iron_golem_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_fox_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(-1.0, 16.5, -3.0))
                .cube(cube([1.0, 5.0], [-3.0, -2.0, -5.0], [8.0, 6.0, 6.0]))
                .child(NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube(
                    [8.0, 1.0],
                    [-3.0, -4.0, -4.0],
                    [2.0, 2.0, 1.0],
                )))
                .child(NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube(
                    [15.0, 1.0],
                    [3.0, -4.0, -4.0],
                    [2.0, 2.0, 1.0],
                )))
                .child(NamedPartSpec::new("nose", PartPose::ZERO).cube(cube(
                    [6.0, 18.0],
                    [-1.0, 2.01, -8.0],
                    [4.0, 2.0, 3.0],
                ))),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 16.0, -6.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([24.0, 15.0], [-3.0, 3.999, -3.5], [6.0, 11.0, 6.0]))
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(-4.0, 15.0, -1.0, -0.05236, 0.0, 0.0),
                )
                .cube(cube([30.0, 0.0], [2.0, 0.0, -1.0], [4.0, 9.0, 5.0])),
            ),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-5.0, 17.5, 7.0)).cube(
                cube_grown([13.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(-1.0, 17.5, 7.0)).cube(
                cube_grown([4.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-5.0, 17.5, 0.0)).cube(
                cube_grown([13.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(-1.0, 17.5, 0.0)).cube(
                cube_grown([4.0, 24.0], [2.0, 0.5, -1.0], [2.0, 6.0, 2.0], 0.001),
            ),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_fox_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_fox_model_tree().mesh(ModelTextureSize {
        width: 48.0,
        height: 32.0,
    })
}
pub fn apply_fox_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    if let Some(head) = tree.part_mut("head") {
        head.pose.rotation.z = state.head_roll_angle;
    }
    let r_hind = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    let l_hind = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
    let r_front = (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
    let l_front = (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
    if let Some(p) = tree.part_mut("right_hind_leg") {
        p.pose.rotation.x = r_hind;
        p.visible = !state.is_sleeping;
    }
    if let Some(p) = tree.part_mut("left_hind_leg") {
        p.pose.rotation.x = l_hind;
        p.visible = !state.is_sleeping;
    }
    if let Some(p) = tree.part_mut("right_front_leg") {
        p.pose.rotation.x = r_front;
        p.visible = !state.is_sleeping;
    }
    if let Some(p) = tree.part_mut("left_front_leg") {
        p.pose.rotation.x = l_front;
        p.visible = !state.is_sleeping;
    }
    if state.is_sitting {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.x = std::f32::consts::PI / 6.0;
            b.pose.offset.y -= 7.0;
            b.pose.offset.z += 3.0;
        }
        if let Some(t) = tree.part_mut("tail") {
            t.pose.rotation.x = std::f32::consts::PI / 4.0;
            t.pose.offset.z -= 1.0;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.y -= 6.5;
            h.pose.offset.z += 2.75;
        }
        if let Some(p) = tree.part_mut("right_front_leg") {
            p.pose.rotation.x = -std::f32::consts::PI / 12.0;
        }
        if let Some(p) = tree.part_mut("left_front_leg") {
            p.pose.rotation.x = -std::f32::consts::PI / 12.0;
        }
        if let Some(p) = tree.part_mut("right_hind_leg") {
            p.pose.rotation.x = -std::f32::consts::PI * 5.0 / 12.0;
            p.pose.offset.y += 4.0;
            p.pose.offset.z -= 0.25;
        }
        if let Some(p) = tree.part_mut("left_hind_leg") {
            p.pose.rotation.x = -std::f32::consts::PI * 5.0 / 12.0;
            p.pose.offset.y += 4.0;
            p.pose.offset.z -= 0.25;
        }
    } else if state.is_sleeping {
        if let Some(b) = tree.part_mut("body") {
            b.pose.rotation.z = -std::f32::consts::PI / 2.0;
            b.pose.offset.y += 5.0;
        }
        if let Some(t) = tree.part_mut("tail") {
            t.pose.rotation.x = -std::f32::consts::PI * 5.0 / 6.0;
        }
        if let Some(h) = tree.part_mut("head") {
            h.pose.offset.x += 2.0;
            h.pose.offset.y += 2.99;
            h.pose.rotation.x = 0.0;
            h.pose.rotation.y = -std::f32::consts::PI * 2.0 / 3.0;
            h.pose.rotation.z = (state.age_in_ticks * 0.027).cos() / 22.0;
        }
    }
    if !state.is_sleeping && !state.is_faceplanted && !state.is_crouching {
        if let Some(h) = tree.part_mut("head") {
            h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
            h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        }
    }
}
pub fn build_fox_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_fox_model_tree();
    tree.reset_pose();
    apply_fox_model_setup_anim_ir(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 48.0,
        height: 32.0,
    })
}
pub fn build_strider_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-4.0, 8.0, 0.0)).cube(cube(
                [0.0, 32.0],
                [-2.0, 0.0, -2.0],
                [4.0, 16.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(4.0, 8.0, 0.0)).cube(cube(
                [0.0, 55.0],
                [-2.0, 0.0, -2.0],
                [4.0, 16.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 1.0, 0.0))
                .cube(cube([0.0, 0.0], [-8.0, -6.0, -8.0], [16.0, 14.0, 16.0]))
                .child(
                    NamedPartSpec::new(
                        "right_bottom_bristle",
                        PartPose::offset_and_rotation(-8.0, 4.0, -8.0, 0.0, 0.0, -1.221731),
                    )
                    .cube(cube_mirrored(
                        [16.0, 65.0],
                        [-12.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "right_middle_bristle",
                        PartPose::offset_and_rotation(-8.0, -1.0, -8.0, 0.0, 0.0, -1.134464),
                    )
                    .cube(cube_mirrored(
                        [16.0, 49.0],
                        [-12.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "right_top_bristle",
                        PartPose::offset_and_rotation(-8.0, -5.0, -8.0, 0.0, 0.0, -0.872665),
                    )
                    .cube(cube_mirrored(
                        [16.0, 33.0],
                        [-12.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_top_bristle",
                        PartPose::offset_and_rotation(8.0, -6.0, -8.0, 0.0, 0.0, 0.872665),
                    )
                    .cube(cube(
                        [16.0, 33.0],
                        [0.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_middle_bristle",
                        PartPose::offset_and_rotation(8.0, -2.0, -8.0, 0.0, 0.0, 1.134464),
                    )
                    .cube(cube(
                        [16.0, 49.0],
                        [0.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_bottom_bristle",
                        PartPose::offset_and_rotation(8.0, 3.0, -8.0, 0.0, 0.0, 1.221731),
                    )
                    .cube(cube(
                        [16.0, 65.0],
                        [0.0, 0.0, 0.0],
                        [12.0, 0.0, 16.0],
                    )),
                ),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_strider_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_strider_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 128.0,
    })
}
pub fn build_camel_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 4.0, 9.5))
                .cube(cube([0.0, 25.0], [-7.5, -12.0, -23.5], [15.0, 12.0, 27.0]))
                .child(
                    NamedPartSpec::new("hump", PartPose::offset(0.0, -12.0, -10.0)).cube(cube(
                        [74.0, 0.0],
                        [-4.5, -5.0, -5.5],
                        [9.0, 5.0, 11.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("tail", PartPose::offset(0.0, -9.0, 3.5)).cube(cube(
                        [122.0, 0.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 14.0, 0.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("head", PartPose::offset(0.0, -3.0, -19.5))
                        .cube(cube([60.0, 24.0], [-3.5, -7.0, -15.0], [7.0, 8.0, 19.0]))
                        .cube(cube([21.0, 0.0], [-3.5, -21.0, -15.0], [7.0, 14.0, 7.0]))
                        .cube(cube([50.0, 0.0], [-2.5, -21.0, -21.0], [5.0, 5.0, 6.0]))
                        .child(
                            NamedPartSpec::new("left_ear", PartPose::offset(2.5, -21.0, -9.5))
                                .cube(cube([45.0, 0.0], [-0.5, 0.5, -1.0], [3.0, 1.0, 2.0])),
                        )
                        .child(
                            NamedPartSpec::new("right_ear", PartPose::offset(-2.5, -21.0, -9.5))
                                .cube(cube([67.0, 0.0], [-2.5, 0.5, -1.0], [3.0, 1.0, 2.0])),
                        ),
                ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(4.9, 1.0, 9.5)).cube(cube(
                [58.0, 16.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.9, 1.0, 9.5)).cube(cube(
                [94.0, 16.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(4.9, 1.0, -10.5)).cube(cube(
                [0.0, 0.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-4.9, 1.0, -10.5)).cube(cube(
                [0.0, 26.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_camel_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_camel_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn apply_camel_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let y_rot = state.y_rot_degrees.clamp(-30.0, 30.0);
    let mut x_rot = state.x_rot_degrees.clamp(-25.0, 45.0);
    if state.jump_cooldown > 0.0 {
        let head_rotation = 45.0 * state.jump_cooldown / 55.0;
        x_rot = (x_rot + head_rotation).clamp(-25.0, 70.0);
    }
    if let Some(head) = tree.part_mut("head") {
        head.pose.rotation.y = y_rot.to_radians();
        head.pose.rotation.x = x_rot.to_radians();
    }
    let speed = state.walk_animation_speed.min(2.0);
    let walk = state.walk_animation_pos * 2.5;
    let leg_rot = (walk * 0.6662).cos() * 1.4 * speed;
    if let Some(leg) = tree.part_mut("right_hind_leg") {
        leg.pose.rotation.x = leg_rot;
    }
    if let Some(leg) = tree.part_mut("left_hind_leg") {
        leg.pose.rotation.x = -leg_rot;
    }
    if let Some(leg) = tree.part_mut("right_front_leg") {
        leg.pose.rotation.x = -leg_rot;
    }
    if let Some(leg) = tree.part_mut("left_front_leg") {
        leg.pose.rotation.x = leg_rot;
    }
}
pub fn build_camel_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_camel_model_tree();
    apply_camel_model_setup_anim_ir(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_nautilus_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("root", PartPose::offset(0.0, 29.0, -6.0))
            .child(
                NamedPartSpec::new("shell", PartPose::offset(0.0, -13.0, 5.0))
                    .cube(cube([0.0, 0.0], [-7.0, -10.0, -7.0], [14.0, 10.0, 16.0]))
                    .cube(cube([0.0, 26.0], [-7.0, 0.0, -7.0], [14.0, 8.0, 20.0]))
                    .cube(cube([48.0, 26.0], [-7.0, 0.0, 6.0], [14.0, 8.0, 0.0])),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -8.5, 12.3))
                    .cube(cube([0.0, 54.0], [-5.0, -4.51, -3.0], [10.0, 8.0, 14.0]))
                    .cube(cube([0.0, 76.0], [-5.0, -4.51, 7.0], [10.0, 8.0, 0.0]))
                    .child(
                        NamedPartSpec::new("upper_mouth", PartPose::offset(0.0, -2.51, 7.0)).cube(
                            cube_grown([54.0, 54.0], [-5.0, -2.0, 0.0], [10.0, 4.0, 4.0], -0.001),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("inner_mouth", PartPose::offset(0.0, -0.51, 7.5))
                            .cube(cube([54.0, 70.0], [-3.0, -2.0, -0.5], [6.0, 4.0, 4.0])),
                    )
                    .child(
                        NamedPartSpec::new("lower_mouth", PartPose::offset(0.0, 1.49, 7.0)).cube(
                            cube_grown([54.0, 62.0], [-5.0, -1.98, 0.0], [10.0, 4.0, 4.0], -0.001),
                        ),
                    ),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_nautilus_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_nautilus_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_blaze_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(NamedPartSpec::new("head", PartPose::ZERO).cube(cube(
                [0.0, 0.0],
                [-4.0, -4.0, -4.0],
                [8.0, 8.0, 8.0],
            )))
            .child(
                NamedPartSpec::new("part0", PartPose::offset(9.0, -1.0, 0.0)).cube(cube(
                    [0.0, 16.0],
                    [0.0, 0.0, 0.0],
                    [2.0, 8.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("part1", PartPose::offset(4.862721, -1.122417, 7.573239))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part2", PartPose::offset(-3.745322, -1.459698, 8.183677))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part3", PartPose::offset(-8.909932, -1.929263, 1.27008))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part4", PartPose::offset(4.949748, 1.583853, 4.949747))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part5", PartPose::offset(-1.490708, 1.198856, 6.839429))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part6", PartPose::offset(-6.560614, 1.010008, 2.440972))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part7", PartPose::offset(-5.598722, 1.063543, -4.201704))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part8", PartPose::offset(4.455032, 11.96017, 2.269953))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part9", PartPose::offset(0.496965, 11.893006, 4.975241))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part10", PartPose::offset(-3.91801, 11.346635, 3.106316))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("part11", PartPose::offset(-4.730784, 10.614252, -1.618542))
                    .cube(cube([0.0, 16.0], [0.0, 0.0, 0.0], [2.0, 8.0, 2.0])),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_blaze_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_blaze_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_blaze_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = -2.0 + ((0.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part0").pose.offset.y = anim_value_0;
    let anim_value_1 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 0.0_f32).cos() * 9.0;
    tree.part_mut_or_panic("part0").pose.offset.x = anim_value_1;
    let anim_value_2 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 0.0_f32).sin() * 9.0;
    tree.part_mut_or_panic("part0").pose.offset.z = anim_value_2;
    let anim_value_3 = -2.0 + ((1.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part1").pose.offset.y = anim_value_3;
    let anim_value_4 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 1.0_f32).cos() * 9.0;
    tree.part_mut_or_panic("part1").pose.offset.x = anim_value_4;
    let anim_value_5 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 1.0_f32).sin() * 9.0;
    tree.part_mut_or_panic("part1").pose.offset.z = anim_value_5;
    let anim_value_6 = -2.0 + ((2.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part2").pose.offset.y = anim_value_6;
    let anim_value_7 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 2.0_f32).cos() * 9.0;
    tree.part_mut_or_panic("part2").pose.offset.x = anim_value_7;
    let anim_value_8 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 2.0_f32).sin() * 9.0;
    tree.part_mut_or_panic("part2").pose.offset.z = anim_value_8;
    let anim_value_9 = -2.0 + ((3.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part3").pose.offset.y = anim_value_9;
    let anim_value_10 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 3.0_f32).cos() * 9.0;
    tree.part_mut_or_panic("part3").pose.offset.x = anim_value_10;
    let anim_value_11 = ((state.age_in_ticks * std::f32::consts::PI * -0.1) + 3.0_f32).sin() * 9.0;
    tree.part_mut_or_panic("part3").pose.offset.z = anim_value_11;
    let anim_value_12 = 2.0 + ((4.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part4").pose.offset.y = anim_value_12;
    let anim_value_13 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 0.0_f32)
        .cos()
        * 7.0;
    tree.part_mut_or_panic("part4").pose.offset.x = anim_value_13;
    let anim_value_14 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 0.0_f32)
        .sin()
        * 7.0;
    tree.part_mut_or_panic("part4").pose.offset.z = anim_value_14;
    let anim_value_15 = 2.0 + ((5.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part5").pose.offset.y = anim_value_15;
    let anim_value_16 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 1.0_f32)
        .cos()
        * 7.0;
    tree.part_mut_or_panic("part5").pose.offset.x = anim_value_16;
    let anim_value_17 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 1.0_f32)
        .sin()
        * 7.0;
    tree.part_mut_or_panic("part5").pose.offset.z = anim_value_17;
    let anim_value_18 = 2.0 + ((6.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part6").pose.offset.y = anim_value_18;
    let anim_value_19 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 2.0_f32)
        .cos()
        * 7.0;
    tree.part_mut_or_panic("part6").pose.offset.x = anim_value_19;
    let anim_value_20 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 2.0_f32)
        .sin()
        * 7.0;
    tree.part_mut_or_panic("part6").pose.offset.z = anim_value_20;
    let anim_value_21 = 2.0 + ((7.0_f32 * 2.0_f32 + state.age_in_ticks) * 0.25).cos();
    tree.part_mut_or_panic("part7").pose.offset.y = anim_value_21;
    let anim_value_22 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 3.0_f32)
        .cos()
        * 7.0;
    tree.part_mut_or_panic("part7").pose.offset.x = anim_value_22;
    let anim_value_23 = (((std::f32::consts::PI / 4.0_f32)
        + state.age_in_ticks * std::f32::consts::PI * 0.03)
        + 3.0_f32)
        .sin()
        * 7.0;
    tree.part_mut_or_panic("part7").pose.offset.z = anim_value_23;
    let anim_value_24 = 11.0 + ((8.0_f32 * 1.5 + state.age_in_ticks) * 0.5).cos();
    tree.part_mut_or_panic("part8").pose.offset.y = anim_value_24;
    let anim_value_25 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 0.0_f32).cos() * 5.0;
    tree.part_mut_or_panic("part8").pose.offset.x = anim_value_25;
    let anim_value_26 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 0.0_f32).sin() * 5.0;
    tree.part_mut_or_panic("part8").pose.offset.z = anim_value_26;
    let anim_value_27 = 11.0 + ((9.0_f32 * 1.5 + state.age_in_ticks) * 0.5).cos();
    tree.part_mut_or_panic("part9").pose.offset.y = anim_value_27;
    let anim_value_28 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 1.0_f32).cos() * 5.0;
    tree.part_mut_or_panic("part9").pose.offset.x = anim_value_28;
    let anim_value_29 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 1.0_f32).sin() * 5.0;
    tree.part_mut_or_panic("part9").pose.offset.z = anim_value_29;
    let anim_value_30 = 11.0 + ((10.0_f32 * 1.5 + state.age_in_ticks) * 0.5).cos();
    tree.part_mut_or_panic("part10").pose.offset.y = anim_value_30;
    let anim_value_31 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 2.0_f32).cos() * 5.0;
    tree.part_mut_or_panic("part10").pose.offset.x = anim_value_31;
    let anim_value_32 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 2.0_f32).sin() * 5.0;
    tree.part_mut_or_panic("part10").pose.offset.z = anim_value_32;
    let anim_value_33 = 11.0 + ((11.0_f32 * 1.5 + state.age_in_ticks) * 0.5).cos();
    tree.part_mut_or_panic("part11").pose.offset.y = anim_value_33;
    let anim_value_34 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 3.0_f32).cos() * 5.0;
    tree.part_mut_or_panic("part11").pose.offset.x = anim_value_34;
    let anim_value_35 =
        ((0.47123894 + state.age_in_ticks * std::f32::consts::PI * -0.05) + 3.0_f32).sin() * 5.0;
    tree.part_mut_or_panic("part11").pose.offset.z = anim_value_35;
    let anim_value_36 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_36;
    let anim_value_37 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_37;
}
pub fn apply_blaze_blaze_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_blaze_model_setup_anim_ir(tree, state);
}
pub fn build_blaze_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_blaze_model_tree();
    apply_blaze_blaze_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_breeze_wind_charge_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("bone", PartPose::offset(0.0, 0.0, 0.0))
            .child(
                NamedPartSpec::new(
                    "wind",
                    PartPose::offset_and_rotation(0.0, 0.0, 0.0, 0.0, -0.7854, 0.0),
                )
                .cube(cube([15.0, 20.0], [-4.0, -1.0, -4.0], [8.0, 2.0, 8.0]))
                .cube(cube([0.0, 9.0], [-3.0, -2.0, -3.0], [6.0, 4.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("wind_charge", PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-2.0, -2.0, -2.0],
                    [4.0, 4.0, 4.0],
                )),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_breeze_wind_charge_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_breeze_wind_charge_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_wind_charge_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = -state.age_in_ticks * 16.0 * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("wind_charge").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.age_in_ticks * 16.0 * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("wind").pose.rotation.y = anim_value_1;
}
pub fn apply_breeze_wind_charge_wind_charge_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_wind_charge_model_setup_anim_ir(tree, state);
}
pub fn build_breeze_wind_charge_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_breeze_wind_charge_model_tree();
    apply_breeze_wind_charge_wind_charge_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_cave_spider_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new(
        "root",
        PartPose {
            offset: Vec3::new(0.0, 7.2048, 0.0),
            rotation: Vec3::splat(0.0),
            scale: Vec3::splat(0.7),
        },
    )
    .child(
        NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -3.0)).cube(cube(
            [32.0, 4.0],
            [-4.0, -4.0, -8.0],
            [8.0, 8.0, 8.0],
        )),
    )
    .child(
        NamedPartSpec::new("body0", PartPose::offset(0.0, 15.0, 0.0)).cube(cube(
            [0.0, 0.0],
            [-3.0, -3.0, -3.0],
            [6.0, 6.0, 6.0],
        )),
    )
    .child(
        NamedPartSpec::new("body1", PartPose::offset(0.0, 15.0, 9.0)).cube(cube(
            [0.0, 12.0],
            [-5.0, -4.0, -6.0],
            [10.0, 8.0, 12.0],
        )),
    )
    .child(
        NamedPartSpec::new(
            "right_hind_leg",
            PartPose::offset_and_rotation(-4.0, 15.0, 2.0, 0.0, 0.785398, -0.785398),
        )
        .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
    )
    .child(
        NamedPartSpec::new(
            "left_hind_leg",
            PartPose::offset_and_rotation(4.0, 15.0, 2.0, 0.0, -0.785398, 0.785398),
        )
        .cube(cube_mirrored(
            [18.0, 0.0],
            [-1.0, -1.0, -1.0],
            [16.0, 2.0, 2.0],
        )),
    )
    .child(
        NamedPartSpec::new(
            "right_middle_hind_leg",
            PartPose::offset_and_rotation(-4.0, 15.0, 1.0, 0.0, 0.392699, -0.581195),
        )
        .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
    )
    .child(
        NamedPartSpec::new(
            "left_middle_hind_leg",
            PartPose::offset_and_rotation(4.0, 15.0, 1.0, 0.0, -0.392699, 0.581195),
        )
        .cube(cube_mirrored(
            [18.0, 0.0],
            [-1.0, -1.0, -1.0],
            [16.0, 2.0, 2.0],
        )),
    )
    .child(
        NamedPartSpec::new(
            "right_middle_front_leg",
            PartPose::offset_and_rotation(-4.0, 15.0, 0.0, 0.0, -0.392699, -0.581195),
        )
        .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
    )
    .child(
        NamedPartSpec::new(
            "left_middle_front_leg",
            PartPose::offset_and_rotation(4.0, 15.0, 0.0, 0.0, 0.392699, 0.581195),
        )
        .cube(cube_mirrored(
            [18.0, 0.0],
            [-1.0, -1.0, -1.0],
            [16.0, 2.0, 2.0],
        )),
    )
    .child(
        NamedPartSpec::new(
            "right_front_leg",
            PartPose::offset_and_rotation(-4.0, 15.0, -1.0, 0.0, -0.785398, -0.785398),
        )
        .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
    )
    .child(
        NamedPartSpec::new(
            "left_front_leg",
            PartPose::offset_and_rotation(4.0, 15.0, -1.0, 0.0, 0.785398, 0.785398),
        )
        .cube(cube_mirrored(
            [18.0, 0.0],
            [-1.0, -1.0, -1.0],
            [16.0, 2.0, 2.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn apply_spider_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_pos = state.walk_animation_pos * 0.6662;
    let animation_speed = state.walk_animation_speed;
    let swing_hind = -((animation_pos * 2.0 + 0.0).cos() * 0.4) * animation_speed;
    let swing_middle_hind =
        -((animation_pos * 2.0 + std::f32::consts::PI).cos() * 0.4) * animation_speed;
    let swing_middle_front =
        -((animation_pos * 2.0 + (std::f32::consts::PI / 2.0_f32)).cos() * 0.4) * animation_speed;
    let swing_front =
        -((animation_pos * 2.0 + (std::f32::consts::PI * 3.0 / 2.0)).cos() * 0.4) * animation_speed;
    let step_hind = ((animation_pos + 0.0).sin() * 0.4).abs() * animation_speed;
    let step_middle_hind =
        ((animation_pos + std::f32::consts::PI).sin() * 0.4).abs() * animation_speed;
    let step_middle_front_hind =
        ((animation_pos + (std::f32::consts::PI / 2.0_f32)).sin() * 0.4).abs() * animation_speed;
    let step_front =
        ((animation_pos + (std::f32::consts::PI * 3.0 / 2.0)).sin() * 0.4).abs() * animation_speed;
    let anim_value_0 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_1;
    let anim_value_2 = swing_hind;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.y += anim_value_2;
    let anim_value_3 = swing_hind;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.y -= anim_value_3;
    let anim_value_4 = swing_middle_hind;
    tree.part_mut_or_panic("right_middle_hind_leg")
        .pose
        .rotation
        .y += anim_value_4;
    let anim_value_5 = swing_middle_hind;
    tree.part_mut_or_panic("left_middle_hind_leg")
        .pose
        .rotation
        .y -= anim_value_5;
    let anim_value_6 = swing_middle_front;
    tree.part_mut_or_panic("right_middle_front_leg")
        .pose
        .rotation
        .y += anim_value_6;
    let anim_value_7 = swing_middle_front;
    tree.part_mut_or_panic("left_middle_front_leg")
        .pose
        .rotation
        .y -= anim_value_7;
    let anim_value_8 = swing_front;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.y += anim_value_8;
    let anim_value_9 = swing_front;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.y -= anim_value_9;
    let anim_value_10 = step_hind;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.z += anim_value_10;
    let anim_value_11 = step_hind;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.z -= anim_value_11;
    let anim_value_12 = step_middle_hind;
    tree.part_mut_or_panic("right_middle_hind_leg")
        .pose
        .rotation
        .z += anim_value_12;
    let anim_value_13 = step_middle_hind;
    tree.part_mut_or_panic("left_middle_hind_leg")
        .pose
        .rotation
        .z -= anim_value_13;
    let anim_value_14 = step_middle_front_hind;
    tree.part_mut_or_panic("right_middle_front_leg")
        .pose
        .rotation
        .z += anim_value_14;
    let anim_value_15 = step_middle_front_hind;
    tree.part_mut_or_panic("left_middle_front_leg")
        .pose
        .rotation
        .z -= anim_value_15;
    let anim_value_16 = step_front;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.z += anim_value_16;
    let anim_value_17 = step_front;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.z -= anim_value_17;
}
pub fn apply_cave_spider_spider_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_spider_model_setup_anim_ir(tree, state);
}
pub fn build_chest_minecart_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_chest_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_chest_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_chest_minecart_minecart_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_chest_minecart_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_chest_minecart_model_tree();
    apply_chest_minecart_minecart_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_chicken_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let flap_angle = ((state.flap).sin() + 1.0) * state.flap_speed;
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    let anim_value_0 = (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_leg").pose.rotation.x = anim_value_0;
    let anim_value_1 =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_leg").pose.rotation.x = anim_value_1;
    let anim_value_2 = flap_angle;
    tree.part_mut_or_panic("right_wing").pose.rotation.z = anim_value_2;
    let anim_value_3 = -flap_angle;
    tree.part_mut_or_panic("left_wing").pose.rotation.z = anim_value_3;
}
pub fn apply_adult_chicken_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_0;
    let anim_value_1 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_1;
}
pub fn apply_chicken_adult_chicken_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_chicken_model_setup_anim_ir(tree, state);
    apply_adult_chicken_model_setup_anim_ir(tree, state);
}
pub fn build_cod_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, -2.0, 0.0],
                [2.0, 4.0, 7.0],
            )),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                [11.0, 0.0],
                [-1.0, -2.0, -3.0],
                [2.0, 4.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("nose", PartPose::offset(0.0, 22.0, -3.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, -2.0, -1.0],
                [2.0, 3.0, 1.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_fin",
                PartPose::offset_and_rotation(-1.0, 23.0, 0.0, 0.0, 0.0, -0.785398),
            )
            .cube(cube([22.0, 1.0], [-2.0, 0.0, -1.0], [2.0, 0.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_fin",
                PartPose::offset_and_rotation(1.0, 23.0, 0.0, 0.0, 0.0, 0.785398),
            )
            .cube(cube([22.0, 4.0], [0.0, 0.0, -1.0], [2.0, 0.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tail_fin", PartPose::offset(0.0, 22.0, 7.0)).cube(cube(
                [22.0, 3.0],
                [0.0, -2.0, 0.0],
                [0.0, 4.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("top_fin", PartPose::offset(0.0, 20.0, 0.0)).cube(cube(
                [20.0, -6.0],
                [0.0, -1.0, -1.0],
                [0.0, 1.0, 6.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn apply_cod_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let amplitude_multiplier = if state.is_in_water { 1.0 } else { 1.5 };
    let anim_value_0 = -amplitude_multiplier * 0.45 * (0.6 * state.age_in_ticks).sin();
    tree.part_mut_or_panic("tail_fin").pose.rotation.y = anim_value_0;
}
pub fn apply_cod_cod_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_cod_model_setup_anim_ir(tree, state);
}
pub fn build_command_block_minecart_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_command_block_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_command_block_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_command_block_minecart_minecart_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_command_block_minecart_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_command_block_minecart_model_tree();
    apply_command_block_minecart_minecart_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_quadruped_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_pos = state.walk_animation_pos;
    let animation_speed = state.walk_animation_speed;
    let anim_value_0 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_0;
    let anim_value_1 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_1;
    let anim_value_2 = (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_2;
    let anim_value_3 =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_3;
    let anim_value_4 =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_4;
    let anim_value_5 = (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_5;
}
pub fn apply_cow_cow_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
}
pub fn build_creeper_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 6.0, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-4.0, -8.0, -4.0],
                    [8.0, 8.0, 8.0],
                )),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 6.0, 0.0)).cube(cube(
                    [16.0, 16.0],
                    [-4.0, 0.0, -2.0],
                    [8.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-2.0, 18.0, 4.0)).cube(cube(
                    [0.0, 16.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 6.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(2.0, 18.0, 4.0)).cube(cube(
                    [0.0, 16.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 6.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-2.0, 18.0, -4.0))
                    .cube(cube([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 6.0, 4.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(2.0, 18.0, -4.0)).cube(cube(
                    [0.0, 16.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 6.0, 4.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn apply_creeper_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    let anim_value_0 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_1;
    let anim_value_2 = (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_2;
    let anim_value_3 =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_3;
    let anim_value_4 =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_4;
    let anim_value_5 = (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_5;
}
pub fn apply_creeper_creeper_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_creeper_model_setup_anim_ir(tree, state);
}
pub fn build_endermite_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("segment0", PartPose::offset(0.0, 21.0, -3.5)).cube(cube(
                [0.0, 0.0],
                [-2.0, 0.0, -1.0],
                [4.0, 3.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment1", PartPose::offset(0.0, 20.0, 0.0)).cube(cube(
                [0.0, 5.0],
                [-3.0, 0.0, -2.5],
                [6.0, 4.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment2", PartPose::offset(0.0, 21.0, 3.0)).cube(cube(
                [0.0, 14.0],
                [-1.5, 0.0, -0.5],
                [3.0, 3.0, 1.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment3", PartPose::offset(0.0, 22.0, 4.0)).cube(cube(
                [0.0, 18.0],
                [-0.5, 0.0, -0.5],
                [1.0, 2.0, 1.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_endermite_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_endermite_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_endermite_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = (state.age_in_ticks * 0.9 + 0.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.01
        * (1.0_f32 + (0.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment0").pose.rotation.y = anim_value_0;
    let anim_value_1 = (state.age_in_ticks * 0.9 + 0.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.1
        * (0.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment0").pose.offset.x = anim_value_1;
    let anim_value_2 = (state.age_in_ticks * 0.9 + 1.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.01
        * (1.0_f32 + (1.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment1").pose.rotation.y = anim_value_2;
    let anim_value_3 = (state.age_in_ticks * 0.9 + 1.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.1
        * (1.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment1").pose.offset.x = anim_value_3;
    let anim_value_4 = (state.age_in_ticks * 0.9 + 2.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.01
        * (1.0_f32 + (2.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment2").pose.rotation.y = anim_value_4;
    let anim_value_5 = (state.age_in_ticks * 0.9 + 2.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.1
        * (2.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment2").pose.offset.x = anim_value_5;
    let anim_value_6 = (state.age_in_ticks * 0.9 + 3.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.01
        * (1.0_f32 + (3.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment3").pose.rotation.y = anim_value_6;
    let anim_value_7 = (state.age_in_ticks * 0.9 + 3.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.1
        * (3.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment3").pose.offset.x = anim_value_7;
}
pub fn apply_endermite_endermite_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_endermite_model_setup_anim_ir(tree, state);
}
pub fn build_endermite_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_endermite_model_tree();
    apply_endermite_endermite_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_furnace_minecart_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_furnace_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_furnace_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_furnace_minecart_minecart_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_furnace_minecart_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_furnace_minecart_model_tree();
    apply_furnace_minecart_minecart_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_ghast_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new(
            "root",
            PartPose {
                offset: Vec3::new(0.0, -84.056, 0.0),
                rotation: Vec3::splat(0.0),
                scale: Vec3::splat(4.5),
            },
        )
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 17.6, 0.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -8.0, -8.0],
                [16.0, 16.0, 16.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle0", PartPose::offset(-3.75, 24.6, -5.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle1", PartPose::offset(0.416667, 24.6, -3.333333))
                .cube(cube([0.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 13.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tentacle2", PartPose::offset(4.583333, 24.6, -1.666667))
                .cube(cube([0.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 9.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tentacle3", PartPose::offset(-6.25, 24.6, 0.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 11.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle4", PartPose::offset(-2.083333, 24.6, 1.666667))
                .cube(cube([0.0, 0.0], [-1.0, 0.0, -1.0], [2.0, 11.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("tentacle5", PartPose::offset(2.083333, 24.6, 3.333333)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 10.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle6", PartPose::offset(-3.75, 24.6, 5.0)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle7", PartPose::offset(0.416667, 24.6, 6.666667)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 9.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("tentacle8", PartPose::offset(4.583333, 24.6, 8.333333)).cube(cube(
                [0.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 12.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_ghast_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_ghast_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_ghast_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = 0.2 * (state.age_in_ticks * 0.3 + 0.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle0").pose.rotation.x = anim_value_0;
    let anim_value_1 = 0.2 * (state.age_in_ticks * 0.3 + 1.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle1").pose.rotation.x = anim_value_1;
    let anim_value_2 = 0.2 * (state.age_in_ticks * 0.3 + 2.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle2").pose.rotation.x = anim_value_2;
    let anim_value_3 = 0.2 * (state.age_in_ticks * 0.3 + 3.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle3").pose.rotation.x = anim_value_3;
    let anim_value_4 = 0.2 * (state.age_in_ticks * 0.3 + 4.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle4").pose.rotation.x = anim_value_4;
    let anim_value_5 = 0.2 * (state.age_in_ticks * 0.3 + 5.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle5").pose.rotation.x = anim_value_5;
    let anim_value_6 = 0.2 * (state.age_in_ticks * 0.3 + 6.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle6").pose.rotation.x = anim_value_6;
    let anim_value_7 = 0.2 * (state.age_in_ticks * 0.3 + 7.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle7").pose.rotation.x = anim_value_7;
    let anim_value_8 = 0.2 * (state.age_in_ticks * 0.3 + 8.0_f32).sin() + 0.4;
    tree.part_mut_or_panic("tentacle8").pose.rotation.x = anim_value_8;
}
pub fn apply_ghast_ghast_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_ghast_model_setup_anim_ir(tree, state);
}
pub fn build_ghast_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_ghast_model_tree();
    apply_ghast_ghast_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_hopper_minecart_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_hopper_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_hopper_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_hopper_minecart_minecart_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_hopper_minecart_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_hopper_minecart_model_tree();
    apply_hopper_minecart_minecart_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_leash_knot_leash_knot_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_leash_knot_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_leash_knot_model_tree();
    apply_leash_knot_leash_knot_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn apply_llama_spit_llama_spit_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_llama_spit_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_llama_spit_model_tree();
    apply_llama_spit_llama_spit_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_magma_cube_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(NamedPartSpec::new("cube0", PartPose::ZERO).cube(cube(
            [0.0, 0.0],
            [-4.0, 16.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("cube1", PartPose::ZERO).cube(cube(
            [0.0, 9.0],
            [-4.0, 17.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("cube2", PartPose::ZERO).cube(cube(
            [0.0, 18.0],
            [-4.0, 18.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("cube3", PartPose::ZERO).cube(cube(
            [0.0, 27.0],
            [-4.0, 19.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("cube4", PartPose::ZERO).cube(cube(
            [32.0, 0.0],
            [-4.0, 20.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("cube5", PartPose::ZERO).cube(cube(
            [32.0, 9.0],
            [-4.0, 21.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("cube6", PartPose::ZERO).cube(cube(
            [32.0, 18.0],
            [-4.0, 22.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("cube7", PartPose::ZERO).cube(cube(
            [32.0, 27.0],
            [-4.0, 23.0, -4.0],
            [8.0, 1.0, 8.0],
        )))
        .child(NamedPartSpec::new("inside_cube", PartPose::ZERO).cube(cube(
            [24.0, 40.0],
            [-2.0, 18.0, -2.0],
            [4.0, 4.0, 4.0],
        )));
    JavaModelPartTree::from_root(root)
}
pub fn apply_magma_cube_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let slime_squish = (0.0_f32).max(state.squish);
    let anim_value_0 = -(4.0_f32 - 0.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube0").pose.offset.y = anim_value_0;
    let anim_value_1 = -(4.0_f32 - 1.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube1").pose.offset.y = anim_value_1;
    let anim_value_2 = -(4.0_f32 - 2.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube2").pose.offset.y = anim_value_2;
    let anim_value_3 = -(4.0_f32 - 3.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube3").pose.offset.y = anim_value_3;
    let anim_value_4 = -(4.0_f32 - 4.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube4").pose.offset.y = anim_value_4;
    let anim_value_5 = -(4.0_f32 - 5.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube5").pose.offset.y = anim_value_5;
    let anim_value_6 = -(4.0_f32 - 6.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube6").pose.offset.y = anim_value_6;
    let anim_value_7 = -(4.0_f32 - 7.0_f32) * slime_squish * 1.7;
    tree.part_mut_or_panic("cube7").pose.offset.y = anim_value_7;
}
pub fn apply_magma_cube_magma_cube_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_magma_cube_model_setup_anim_ir(tree, state);
}
pub fn apply_minecart_minecart_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_minecart_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_minecart_model_tree();
    apply_minecart_minecart_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_mooshroom_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 4.0, -8.0))
                    .cube(cube([0.0, 0.0], [-4.0, -4.0, -6.0], [8.0, 8.0, 6.0]))
                    .cube(cube([1.0, 33.0], [-3.0, 1.0, -7.0], [6.0, 3.0, 1.0]))
                    .cube(cube([22.0, 0.0], [-5.0, -5.0, -5.0], [1.0, 3.0, 1.0]))
                    .cube(cube([22.0, 0.0], [4.0, -5.0, -5.0], [1.0, 3.0, 1.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 5.0, 2.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([18.0, 4.0], [-6.0, -10.0, -7.0], [12.0, 18.0, 10.0]))
                .cube(cube([52.0, 0.0], [-2.0, 2.0, -8.0], [4.0, 6.0, 1.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 12.0, 7.0)).cube(cube(
                    [0.0, 16.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 12.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 12.0, 7.0)).cube(
                    cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 12.0, -5.0))
                    .cube(cube([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 12.0, -5.0)).cube(
                    cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_mooshroom_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_mooshroom_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_mooshroom_cow_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
}
pub fn build_mooshroom_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_mooshroom_model_tree();
    apply_mooshroom_cow_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_pig_pig_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
}
pub fn build_salmon_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body_front", PartPose::offset(0.0, 20.0, -7.2))
                .cube(cube([0.0, 0.0], [-1.5, -2.5, 0.0], [3.0, 5.0, 8.0]))
                .child(
                    NamedPartSpec::new("top_front_fin", PartPose::offset(0.0, -4.5, 5.0))
                        .cube(cube([2.0, 1.0], [0.0, 0.0, 0.0], [0.0, 2.0, 3.0])),
                ),
        )
        .child(
            NamedPartSpec::new("body_back", PartPose::offset(0.0, 20.0, 0.8))
                .cube(cube([0.0, 13.0], [-1.5, -2.5, 0.0], [3.0, 5.0, 8.0]))
                .child(
                    NamedPartSpec::new("back_fin", PartPose::offset(0.0, 0.0, 8.0)).cube(cube(
                        [20.0, 10.0],
                        [0.0, -2.5, 0.0],
                        [0.0, 5.0, 6.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("top_back_fin", PartPose::offset(0.0, -4.5, -1.0))
                        .cube(cube([0.0, 2.0], [0.0, 0.0, 0.0], [0.0, 2.0, 4.0])),
                ),
        )
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 20.0, -7.2)).cube(cube(
                [22.0, 0.0],
                [-1.0, -2.0, -3.0],
                [2.0, 4.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_fin",
                PartPose::offset_and_rotation(-1.5, 21.5, -7.2, 0.0, 0.0, -0.785398),
            )
            .cube(cube([-4.0, 0.0], [-2.0, 0.0, 0.0], [2.0, 0.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_fin",
                PartPose::offset_and_rotation(1.5, 21.5, -7.2, 0.0, 0.0, 0.785398),
            )
            .cube(cube([0.0, 0.0], [0.0, 0.0, 0.0], [2.0, 0.0, 2.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn apply_salmon_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let mut amplitude_multiplier = 1.0;
    let mut angle_multiplier = 1.0;
    if !state.is_in_water {
        amplitude_multiplier = 1.3;
    }
    if !state.is_in_water {
        angle_multiplier = 1.7;
    }
    let anim_value_0 =
        -amplitude_multiplier * 0.25 * (angle_multiplier * 0.6 * state.age_in_ticks).sin();
    tree.part_mut_or_panic("body_back").pose.rotation.y = anim_value_0;
}
pub fn apply_salmon_salmon_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_salmon_model_setup_anim_ir(tree, state);
}
pub fn build_shulker_bullet_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("main", PartPose::ZERO)
            .cube(cube([0.0, 0.0], [-4.0, -4.0, -1.0], [8.0, 8.0, 2.0]))
            .cube(cube([0.0, 10.0], [-1.0, -4.0, -4.0], [2.0, 8.0, 8.0]))
            .cube(cube([20.0, 0.0], [-4.0, -1.0, -4.0], [8.0, 2.0, 8.0])),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_shulker_bullet_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_shulker_bullet_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_shulker_bullet_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("main").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("main").pose.rotation.x = anim_value_1;
}
pub fn apply_shulker_bullet_shulker_bullet_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_shulker_bullet_model_setup_anim_ir(tree, state);
}
pub fn build_shulker_bullet_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_shulker_bullet_model_tree();
    apply_shulker_bullet_shulker_bullet_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_silverfish_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("segment0", PartPose::offset(0.0, 22.0, -3.5)).cube(cube(
                [0.0, 0.0],
                [-1.5, 0.0, -1.0],
                [3.0, 2.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment1", PartPose::offset(0.0, 21.0, -1.5)).cube(cube(
                [0.0, 4.0],
                [-2.0, 0.0, -1.0],
                [4.0, 3.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment2", PartPose::offset(0.0, 20.0, 1.0)).cube(cube(
                [0.0, 9.0],
                [-3.0, 0.0, -1.5],
                [6.0, 4.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment3", PartPose::offset(0.0, 21.0, 4.0)).cube(cube(
                [0.0, 16.0],
                [-1.5, 0.0, -1.5],
                [3.0, 3.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment4", PartPose::offset(0.0, 22.0, 7.0)).cube(cube(
                [0.0, 22.0],
                [-1.0, 0.0, -1.5],
                [2.0, 2.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment5", PartPose::offset(0.0, 23.0, 9.5)).cube(cube(
                [11.0, 0.0],
                [-1.0, 0.0, -1.0],
                [2.0, 1.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("segment6", PartPose::offset(0.0, 23.0, 11.5)).cube(cube(
                [13.0, 4.0],
                [-0.5, 0.0, -1.0],
                [1.0, 1.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("layer0", PartPose::offset(0.0, 16.0, 1.0)).cube(cube(
                [20.0, 0.0],
                [-5.0, 0.0, -1.5],
                [10.0, 8.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("layer1", PartPose::offset(0.0, 20.0, 7.0)).cube(cube(
                [20.0, 11.0],
                [-3.0, 0.0, -1.5],
                [6.0, 4.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("layer2", PartPose::offset(0.0, 19.0, -1.5)).cube(cube(
                [20.0, 18.0],
                [-3.0, 0.0, -1.5],
                [6.0, 5.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_silverfish_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_silverfish_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_silverfish_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = (state.age_in_ticks * 0.9 + 0.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.05
        * (1.0_f32 + (0.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment0").pose.rotation.y = anim_value_0;
    let anim_value_1 = (state.age_in_ticks * 0.9 + 0.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.2
        * (0.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment0").pose.offset.x = anim_value_1;
    let anim_value_2 = (state.age_in_ticks * 0.9 + 1.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.05
        * (1.0_f32 + (1.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment1").pose.rotation.y = anim_value_2;
    let anim_value_3 = (state.age_in_ticks * 0.9 + 1.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.2
        * (1.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment1").pose.offset.x = anim_value_3;
    let anim_value_4 = (state.age_in_ticks * 0.9 + 2.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.05
        * (1.0_f32 + (2.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment2").pose.rotation.y = anim_value_4;
    let anim_value_5 = (state.age_in_ticks * 0.9 + 2.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.2
        * (2.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment2").pose.offset.x = anim_value_5;
    let anim_value_6 = (state.age_in_ticks * 0.9 + 3.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.05
        * (1.0_f32 + (3.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment3").pose.rotation.y = anim_value_6;
    let anim_value_7 = (state.age_in_ticks * 0.9 + 3.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.2
        * (3.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment3").pose.offset.x = anim_value_7;
    let anim_value_8 = (state.age_in_ticks * 0.9 + 4.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.05
        * (1.0_f32 + (4.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment4").pose.rotation.y = anim_value_8;
    let anim_value_9 = (state.age_in_ticks * 0.9 + 4.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.2
        * (4.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment4").pose.offset.x = anim_value_9;
    let anim_value_10 = (state.age_in_ticks * 0.9 + 5.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.05
        * (1.0_f32 + (5.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment5").pose.rotation.y = anim_value_10;
    let anim_value_11 = (state.age_in_ticks * 0.9 + 5.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.2
        * (5.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment5").pose.offset.x = anim_value_11;
    let anim_value_12 = (state.age_in_ticks * 0.9 + 6.0_f32 * 0.15 * std::f32::consts::PI).cos()
        * std::f32::consts::PI
        * 0.05
        * (1.0_f32 + (6.0_f32 - 2.0_f32).abs());
    tree.part_mut_or_panic("segment6").pose.rotation.y = anim_value_12;
    let anim_value_13 = (state.age_in_ticks * 0.9 + 6.0_f32 * 0.15 * std::f32::consts::PI).sin()
        * std::f32::consts::PI
        * 0.2
        * (6.0_f32 - 2.0_f32).abs();
    tree.part_mut_or_panic("segment6").pose.offset.x = anim_value_13;
    let anim_value_14 = tree.part_or_panic("segment2").pose.rotation.y;
    tree.part_mut_or_panic("layer0").pose.rotation.y = anim_value_14;
    let anim_value_15 = tree.part_or_panic("segment4").pose.rotation.y;
    tree.part_mut_or_panic("layer1").pose.rotation.y = anim_value_15;
    let anim_value_16 = tree.part_or_panic("segment4").pose.offset.x;
    tree.part_mut_or_panic("layer1").pose.offset.x = anim_value_16;
    let anim_value_17 = tree.part_or_panic("segment1").pose.rotation.y;
    tree.part_mut_or_panic("layer2").pose.rotation.y = anim_value_17;
    let anim_value_18 = tree.part_or_panic("segment1").pose.offset.x;
    tree.part_mut_or_panic("layer2").pose.offset.x = anim_value_18;
}
pub fn apply_silverfish_silverfish_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_silverfish_model_setup_anim_ir(tree, state);
}
pub fn build_silverfish_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_silverfish_model_tree();
    apply_silverfish_silverfish_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_slime_slime_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_snow_golem_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 4.0, 0.0)).cube(cube_grown(
                    [0.0, 0.0],
                    [-4.0, -8.0, -4.0],
                    [8.0, 8.0, 8.0],
                    -0.5,
                )),
            )
            .child(
                NamedPartSpec::new(
                    "left_arm",
                    PartPose::offset_and_rotation(5.0, 6.0, 1.0, 0.0, 0.0, 1.0),
                )
                .cube(cube_grown(
                    [32.0, 0.0],
                    [-1.0, 0.0, -1.0],
                    [12.0, 2.0, 2.0],
                    -0.5,
                )),
            )
            .child(
                NamedPartSpec::new(
                    "right_arm",
                    PartPose::offset_and_rotation(-5.0, 6.0, -1.0, 0.0, 3.141593, -1.0),
                )
                .cube(cube_grown(
                    [32.0, 0.0],
                    [-1.0, 0.0, -1.0],
                    [12.0, 2.0, 2.0],
                    -0.5,
                )),
            )
            .child(
                NamedPartSpec::new("upper_body", PartPose::offset(0.0, 13.0, 0.0)).cube(
                    cube_grown([0.0, 16.0], [-5.0, -10.0, -5.0], [10.0, 10.0, 10.0], -0.5),
                ),
            )
            .child(
                NamedPartSpec::new("lower_body", PartPose::offset(0.0, 24.0, 0.0)).cube(
                    cube_grown([0.0, 36.0], [-6.0, -12.0, -6.0], [12.0, 12.0, 12.0], -0.5),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn apply_snow_golem_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let sin = (tree.part_or_panic("upper_body").pose.rotation.y).sin();
    let cos = (tree.part_or_panic("upper_body").pose.rotation.y).cos();
    let anim_value_0 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_1;
    let anim_value_2 = state.y_rot_degrees * (std::f32::consts::PI / 180.0) * 0.25;
    tree.part_mut_or_panic("upper_body").pose.rotation.y = anim_value_2;
    let anim_value_3 = tree.part_or_panic("upper_body").pose.rotation.y;
    tree.part_mut_or_panic("left_arm").pose.rotation.y = anim_value_3;
    let anim_value_4 = tree.part_or_panic("upper_body").pose.rotation.y + std::f32::consts::PI;
    tree.part_mut_or_panic("right_arm").pose.rotation.y = anim_value_4;
    let anim_value_5 = cos * 5.0;
    tree.part_mut_or_panic("left_arm").pose.offset.x = anim_value_5;
    let anim_value_6 = -sin * 5.0;
    tree.part_mut_or_panic("left_arm").pose.offset.z = anim_value_6;
    let anim_value_7 = -cos * 5.0;
    tree.part_mut_or_panic("right_arm").pose.offset.x = anim_value_7;
    let anim_value_8 = sin * 5.0;
    tree.part_mut_or_panic("right_arm").pose.offset.z = anim_value_8;
}
pub fn apply_snow_golem_snow_golem_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_snow_golem_model_setup_anim_ir(tree, state);
}
pub fn build_spawner_minecart_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_spawner_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_spawner_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_spawner_minecart_minecart_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_spawner_minecart_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_spawner_minecart_model_tree();
    apply_spawner_minecart_minecart_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_spider_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -3.0)).cube(cube(
                [32.0, 4.0],
                [-4.0, -4.0, -8.0],
                [8.0, 8.0, 8.0],
            )),
        )
        .child(
            NamedPartSpec::new("body0", PartPose::offset(0.0, 15.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-3.0, -3.0, -3.0],
                [6.0, 6.0, 6.0],
            )),
        )
        .child(
            NamedPartSpec::new("body1", PartPose::offset(0.0, 15.0, 9.0)).cube(cube(
                [0.0, 12.0],
                [-5.0, -4.0, -6.0],
                [10.0, 8.0, 12.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_hind_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, 2.0, 0.0, 0.785398, -0.785398),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_hind_leg",
                PartPose::offset_and_rotation(4.0, 15.0, 2.0, 0.0, -0.785398, 0.785398),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_middle_hind_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, 1.0, 0.0, 0.392699, -0.581195),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_middle_hind_leg",
                PartPose::offset_and_rotation(4.0, 15.0, 1.0, 0.0, -0.392699, 0.581195),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_middle_front_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, 0.0, 0.0, -0.392699, -0.581195),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_middle_front_leg",
                PartPose::offset_and_rotation(4.0, 15.0, 0.0, 0.0, 0.392699, 0.581195),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "right_front_leg",
                PartPose::offset_and_rotation(-4.0, 15.0, -1.0, 0.0, -0.785398, -0.785398),
            )
            .cube(cube([18.0, 0.0], [-15.0, -1.0, -1.0], [16.0, 2.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_front_leg",
                PartPose::offset_and_rotation(4.0, 15.0, -1.0, 0.0, 0.785398, 0.785398),
            )
            .cube(cube_mirrored(
                [18.0, 0.0],
                [-1.0, -1.0, -1.0],
                [16.0, 2.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn apply_spider_spider_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_spider_model_setup_anim_ir(tree, state);
}
pub fn apply_sulfur_cube_sulfur_cube_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_sulfur_cube_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_sulfur_cube_model_tree();
    apply_sulfur_cube_sulfur_cube_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_frog_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::offset(0.0, 24.0, 0.0))
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -2.0, 4.0))
                    .cube(cube([3.0, 1.0], [-3.5, -2.0, -8.0], [7.0, 3.0, 9.0]))
                    .cube(cube([23.0, 22.0], [-3.5, -1.0, -8.0], [7.0, 0.0, 9.0]))
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, -2.0, -1.0))
                            .cube(cube([23.0, 13.0], [-3.5, -1.0, -7.0], [7.0, 0.0, 9.0]))
                            .cube(cube([0.0, 13.0], [-3.5, -2.0, -7.0], [7.0, 3.0, 9.0]))
                            .child(
                                NamedPartSpec::new("eyes", PartPose::offset(-0.5, 0.0, 2.0))
                                    .child(
                                        NamedPartSpec::new(
                                            "right_eye",
                                            PartPose::offset(-1.5, -3.0, -6.5),
                                        )
                                        .cube(cube(
                                            [0.0, 0.0],
                                            [-1.5, -1.0, -1.5],
                                            [3.0, 2.0, 3.0],
                                        )),
                                    )
                                    .child(
                                        NamedPartSpec::new(
                                            "left_eye",
                                            PartPose::offset(2.5, -3.0, -6.5),
                                        )
                                        .cube(cube(
                                            [0.0, 5.0],
                                            [-1.5, -1.0, -1.5],
                                            [3.0, 2.0, 3.0],
                                        )),
                                    ),
                            ),
                    )
                    .child(
                        NamedPartSpec::new("croaking_body", PartPose::offset(0.0, -1.0, -5.0))
                            .visible(false)
                            .cube(cube_grown(
                                [26.0, 5.0],
                                [-3.5, -0.1, -2.9],
                                [7.0, 2.0, 3.0],
                                -0.1,
                            )),
                    )
                    .child(
                        NamedPartSpec::new("tongue", PartPose::offset(0.0, -1.01, 1.0)).cube(cube(
                            [17.0, 13.0],
                            [-2.0, 0.0, -7.1],
                            [4.0, 0.0, 7.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("left_arm", PartPose::offset(4.0, -1.0, -6.5))
                            .cube(cube([0.0, 32.0], [-1.0, 0.0, -1.0], [2.0, 3.0, 3.0]))
                            .child(
                                NamedPartSpec::new("left_hand", PartPose::offset(0.0, 3.0, -1.0))
                                    .cube(cube([18.0, 40.0], [-4.0, 0.01, -4.0], [8.0, 0.0, 8.0])),
                            ),
                    )
                    .child(
                        NamedPartSpec::new("right_arm", PartPose::offset(-4.0, -1.0, -6.5))
                            .cube(cube([0.0, 38.0], [-1.0, 0.0, -1.0], [2.0, 3.0, 3.0]))
                            .child(
                                NamedPartSpec::new("right_hand", PartPose::offset(0.0, 3.0, 0.0))
                                    .cube(cube([2.0, 40.0], [-4.0, 0.01, -5.0], [8.0, 0.0, 8.0])),
                            ),
                    ),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(3.5, -3.0, 4.0))
                    .cube(cube([14.0, 25.0], [-1.0, 0.0, -2.0], [3.0, 3.0, 4.0]))
                    .child(
                        NamedPartSpec::new("left_foot", PartPose::offset(2.0, 3.0, 0.0))
                            .cube(cube([2.0, 32.0], [-4.0, 0.01, -4.0], [8.0, 0.0, 8.0])),
                    ),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-3.5, -3.0, 4.0))
                    .cube(cube([0.0, 25.0], [-2.0, 0.0, -2.0], [3.0, 3.0, 4.0]))
                    .child(
                        NamedPartSpec::new("right_foot", PartPose::offset(-2.0, 3.0, 0.0))
                            .cube(cube([18.0, 32.0], [-4.0, 0.01, -4.0], [8.0, 0.0, 8.0])),
                    ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn apply_frog_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let speed = if state.is_swimming {
        state.walk_animation_speed.min(1.0)
    } else {
        state.walk_animation_speed.min(1.5)
    };
    let pos = state.walk_animation_pos * 2.5;
    let scale = if state.is_swimming { 1.0 } else { 1.5 };
    let leg_rot = (pos * 0.6662).cos() * scale * speed;
    if let Some(leg) = tree.part_mut("left_leg") {
        leg.pose.rotation.x = leg_rot;
    }
    if let Some(leg) = tree.part_mut("right_leg") {
        leg.pose.rotation.x = -leg_rot;
    }
}
pub fn build_frog_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_frog_model_tree().mesh(ModelTextureSize {
        width: 48.0,
        height: 48.0,
    })
}
pub fn build_frog_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_frog_model_tree();
    apply_frog_model_setup_anim_ir(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 48.0,
        height: 48.0,
    })
}
pub fn build_tadpole_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, -3.0)).cube(cube(
                [0.0, 0.0],
                [-1.5, -1.0, 0.0],
                [3.0, 2.0, 3.0],
            )),
        )
        .child(
            NamedPartSpec::new("tail", PartPose::offset(0.0, 22.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 2.0, 7.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn apply_tadpole_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let amplitude_multiplier = if state.is_in_water { 1.0 } else { 1.5 };
    let anim_value_0 = -amplitude_multiplier * 0.25 * (0.3 * state.age_in_ticks).sin();
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_0;
}
pub fn apply_tadpole_tadpole_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_tadpole_model_setup_anim_ir(tree, state);
}
pub fn build_tnt_minecart_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 4.0, 0.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 10.0], [-10.0, -8.0, -1.0], [20.0, 16.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(-9.0, 4.0, 0.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(9.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "left",
                PartPose::offset_and_rotation(0.0, 4.0, -7.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 0.0], [-8.0, -9.0, -1.0], [16.0, 8.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("right", PartPose::offset(0.0, 4.0, 7.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -9.0, -1.0],
                [16.0, 8.0, 2.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_tnt_minecart_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_tnt_minecart_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_tnt_minecart_minecart_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_tnt_minecart_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_tnt_minecart_model_tree();
    apply_tnt_minecart_minecart_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_trident_trident_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
}
pub fn build_trident_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_trident_model_tree();
    apply_trident_trident_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn build_wind_charge_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("bone", PartPose::offset(0.0, 0.0, 0.0))
            .child(
                NamedPartSpec::new(
                    "wind",
                    PartPose::offset_and_rotation(0.0, 0.0, 0.0, 0.0, -0.7854, 0.0),
                )
                .cube(cube([15.0, 20.0], [-4.0, -1.0, -4.0], [8.0, 2.0, 8.0]))
                .cube(cube([0.0, 9.0], [-3.0, -2.0, -3.0], [6.0, 4.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("wind_charge", PartPose::offset(0.0, 0.0, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-2.0, -2.0, -2.0],
                    [4.0, 4.0, 4.0],
                )),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_wind_charge_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wind_charge_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_wind_charge_wind_charge_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_wind_charge_model_setup_anim_ir(tree, state);
}
pub fn build_wind_charge_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wind_charge_model_tree();
    apply_wind_charge_wind_charge_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_wither_skull_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("head", PartPose::ZERO).cube(cube(
            [0.0, 35.0],
            [-4.0, -8.0, -4.0],
            [8.0, 8.0, 8.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_wither_skull_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wither_skull_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_skull_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_1;
}
pub fn apply_wither_skull_skull_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_skull_model_setup_anim_ir(tree, state);
}
pub fn build_wither_skull_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wither_skull_model_tree();
    apply_wither_skull_skull_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_acacia_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_acacia_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_acacia_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_acacia_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_acacia_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_acacia_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_bamboo_chest_raft_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, -2.1, 1.0, 1.5708, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -11.0, -4.0], [28.0, 20.0, 4.0]))
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -8.0], [28.0, 16.0, 4.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -4.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([0.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([0.0, 24.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -4.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([40.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([40.0, 24.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -10.1, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -14.1, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -11.1, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_bamboo_chest_raft_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_bamboo_chest_raft_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_bamboo_raft_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, -2.1, 1.0, 1.5708, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -11.0, -4.0], [28.0, 20.0, 4.0]))
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -8.0], [28.0, 16.0, 4.0])),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -4.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([0.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([0.0, 24.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -4.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([40.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([40.0, 24.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_bamboo_raft_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_bamboo_raft_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_bee_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("bone", PartPose::offset(0.0, 19.0, 0.0))
            .child(
                NamedPartSpec::new("body", PartPose::ZERO)
                    .cube(cube([0.0, 0.0], [-3.5, -4.0, -5.0], [7.0, 7.0, 10.0]))
                    .child(NamedPartSpec::new("stinger", PartPose::ZERO).cube(cube(
                        [26.0, 7.0],
                        [0.0, -1.0, 5.0],
                        [0.0, 1.0, 2.0],
                    )))
                    .child(
                        NamedPartSpec::new("left_antenna", PartPose::offset(0.0, -2.0, -5.0))
                            .cube(cube([2.0, 0.0], [1.5, -2.0, -3.0], [1.0, 2.0, 3.0])),
                    )
                    .child(
                        NamedPartSpec::new("right_antenna", PartPose::offset(0.0, -2.0, -5.0))
                            .cube(cube([2.0, 3.0], [-2.5, -2.0, -3.0], [1.0, 2.0, 3.0])),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "right_wing",
                    PartPose::offset_and_rotation(-1.5, -4.0, -3.0, 0.0, -0.2618, 0.0),
                )
                .cube(cube_grown(
                    [0.0, 18.0],
                    [-9.0, 0.0, 0.0],
                    [9.0, 0.0, 6.0],
                    0.001,
                )),
            )
            .child(
                NamedPartSpec::new(
                    "left_wing",
                    PartPose::offset_and_rotation(1.5, -4.0, -3.0, 0.0, 0.2618, 0.0),
                )
                .cube(cube_grown(
                    [0.0, 18.0],
                    [0.0, 0.0, 0.0],
                    [9.0, 0.0, 6.0],
                    0.001,
                )),
            )
            .child(
                NamedPartSpec::new("front_legs", PartPose::offset(1.5, 3.0, -2.0)).cube(cube(
                    [26.0, 1.0],
                    [-5.0, 0.0, 0.0],
                    [7.0, 2.0, 0.0],
                )),
            )
            .child(
                NamedPartSpec::new("middle_legs", PartPose::offset(1.5, 3.0, 0.0)).cube(cube(
                    [26.0, 3.0],
                    [-5.0, 0.0, 0.0],
                    [7.0, 2.0, 0.0],
                )),
            )
            .child(
                NamedPartSpec::new("back_legs", PartPose::offset(1.5, 3.0, 2.0)).cube(cube(
                    [26.0, 5.0],
                    [-5.0, 0.0, 0.0],
                    [7.0, 2.0, 0.0],
                )),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_bee_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_bee_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_birch_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_birch_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_birch_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_birch_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_birch_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_birch_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_camel_husk_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 4.0, 9.5))
                .cube(cube([0.0, 25.0], [-7.5, -12.0, -23.5], [15.0, 12.0, 27.0]))
                .child(
                    NamedPartSpec::new("hump", PartPose::offset(0.0, -12.0, -10.0)).cube(cube(
                        [74.0, 0.0],
                        [-4.5, -5.0, -5.5],
                        [9.0, 5.0, 11.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("tail", PartPose::offset(0.0, -9.0, 3.5)).cube(cube(
                        [122.0, 0.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 14.0, 0.0],
                    )),
                )
                .child(
                    NamedPartSpec::new("head", PartPose::offset(0.0, -3.0, -19.5))
                        .cube(cube([60.0, 24.0], [-3.5, -7.0, -15.0], [7.0, 8.0, 19.0]))
                        .cube(cube([21.0, 0.0], [-3.5, -21.0, -15.0], [7.0, 14.0, 7.0]))
                        .cube(cube([50.0, 0.0], [-2.5, -21.0, -21.0], [5.0, 5.0, 6.0]))
                        .child(
                            NamedPartSpec::new("left_ear", PartPose::offset(2.5, -21.0, -9.5))
                                .cube(cube([45.0, 0.0], [-0.5, 0.5, -1.0], [3.0, 1.0, 2.0])),
                        )
                        .child(
                            NamedPartSpec::new("right_ear", PartPose::offset(-2.5, -21.0, -9.5))
                                .cube(cube([67.0, 0.0], [-2.5, 0.5, -1.0], [3.0, 1.0, 2.0])),
                        ),
                ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(4.9, 1.0, 9.5)).cube(cube(
                [58.0, 16.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.9, 1.0, 9.5)).cube(cube(
                [94.0, 16.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(4.9, 1.0, -10.5)).cube(cube(
                [0.0, 0.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-4.9, 1.0, -10.5)).cube(cube(
                [0.0, 26.0],
                [-2.5, 2.0, -2.5],
                [5.0, 21.0, 5.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_camel_husk_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_camel_husk_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_cherry_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_cherry_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cherry_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_cherry_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_cherry_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cherry_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_dark_oak_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_dark_oak_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_dark_oak_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_dark_oak_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_dark_oak_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_dark_oak_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_jungle_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_jungle_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_jungle_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_jungle_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_jungle_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_jungle_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_mangrove_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_mangrove_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_mangrove_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_mangrove_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_mangrove_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_mangrove_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_oak_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_oak_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_oak_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_oak_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_oak_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_oak_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_pale_oak_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_pale_oak_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_pale_oak_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_pale_oak_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_pale_oak_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_pale_oak_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_spruce_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_spruce_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_spruce_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_spruce_chest_boat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new(
                "bottom",
                PartPose::offset_and_rotation(0.0, 3.0, 1.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-14.0, -9.0, -3.0], [28.0, 16.0, 3.0])),
        )
        .child(
            NamedPartSpec::new(
                "back",
                PartPose::offset_and_rotation(-15.0, 4.0, 4.0, 0.0, 4.712389, 0.0),
            )
            .cube(cube([0.0, 19.0], [-13.0, -7.0, -1.0], [18.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "front",
                PartPose::offset_and_rotation(15.0, 4.0, 0.0, 0.0, 1.570796, 0.0),
            )
            .cube(cube([0.0, 27.0], [-8.0, -7.0, -1.0], [16.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new(
                "right",
                PartPose::offset_and_rotation(0.0, 4.0, -9.0, 0.0, 3.141593, 0.0),
            )
            .cube(cube([0.0, 35.0], [-14.0, -7.0, -1.0], [28.0, 6.0, 2.0])),
        )
        .child(
            NamedPartSpec::new("left", PartPose::offset(0.0, 4.0, 9.0)).cube(cube(
                [0.0, 43.0],
                [-14.0, -7.0, -1.0],
                [28.0, 6.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "left_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, 9.0, 0.0, 0.0, 0.19635),
            )
            .cube(cube([62.0, 0.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 0.0], [-1.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "right_paddle",
                PartPose::offset_and_rotation(3.0, -5.0, -9.0, 0.0, 3.141593, 0.19635),
            )
            .cube(cube([62.0, 20.0], [-1.0, 0.0, -5.0], [2.0, 2.0, 18.0]))
            .cube(cube([62.0, 20.0], [0.001, -3.0, 8.0], [1.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_bottom",
                PartPose::offset_and_rotation(-2.0, -5.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [12.0, 8.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lid",
                PartPose::offset_and_rotation(-2.0, -9.0, -6.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [12.0, 4.0, 12.0])),
        )
        .child(
            NamedPartSpec::new(
                "chest_lock",
                PartPose::offset_and_rotation(-1.0, -6.0, -1.0, 0.0, -1.570796, 0.0),
            )
            .cube(cube([0.0, 59.0], [0.0, 0.0, 0.0], [2.0, 4.0, 1.0])),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_spruce_chest_boat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_spruce_chest_boat_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_wolf_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(-1.0, 13.5, -7.0)).child(
                NamedPartSpec::new("real_head", PartPose::ZERO)
                    .cube(cube([0.0, 0.0], [-2.0, -3.0, -2.0], [6.0, 6.0, 4.0]))
                    .cube(cube([16.0, 14.0], [-2.0, -5.0, 0.0], [2.0, 2.0, 1.0]))
                    .cube(cube([16.0, 14.0], [2.0, -5.0, 0.0], [2.0, 2.0, 1.0]))
                    .cube(cube([0.0, 10.0], [-0.5, -0.001, -5.0], [3.0, 3.0, 4.0])),
            ),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 14.0, 2.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([18.0, 14.0], [-3.0, -2.0, -3.0], [6.0, 9.0, 6.0])),
        )
        .child(
            NamedPartSpec::new(
                "upper_body",
                PartPose::offset_and_rotation(-1.0, 14.0, -3.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([21.0, 0.0], [-3.0, -3.0, -3.0], [8.0, 6.0, 7.0])),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-2.5, 16.0, 7.0)).cube(
                cube_mirrored([0.0, 18.0], [0.0, 0.0, -1.0], [2.0, 8.0, 2.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(0.5, 16.0, 7.0)).cube(cube(
                [0.0, 18.0],
                [0.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-2.5, 16.0, -4.0)).cube(
                cube_mirrored([0.0, 18.0], [0.0, 0.0, -1.0], [2.0, 8.0, 2.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(0.5, 16.0, -4.0)).cube(cube(
                [0.0, 18.0],
                [0.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "tail",
                PartPose::offset_and_rotation(-1.0, 12.0, 8.0, 0.628319, 0.0, 0.0),
            )
            .child(NamedPartSpec::new("real_tail", PartPose::ZERO).cube(cube(
                [9.0, 18.0],
                [0.0, 0.0, -1.0],
                [2.0, 8.0, 2.0],
            ))),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_wolf_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wolf_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_zombie_nautilus_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("root", PartPose::offset(0.0, 29.0, -6.0))
            .child(
                NamedPartSpec::new("shell", PartPose::offset(0.0, -13.0, 5.0))
                    .cube(cube([0.0, 0.0], [-7.0, -10.0, -7.0], [14.0, 10.0, 16.0]))
                    .cube(cube([0.0, 26.0], [-7.0, 0.0, -7.0], [14.0, 8.0, 20.0]))
                    .cube(cube([48.0, 26.0], [-7.0, 0.0, 6.0], [14.0, 8.0, 0.0])),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -8.5, 12.3))
                    .cube(cube([0.0, 54.0], [-5.0, -4.51, -3.0], [10.0, 8.0, 14.0]))
                    .cube(cube([0.0, 76.0], [-5.0, -4.51, 7.0], [10.0, 8.0, 0.0]))
                    .child(
                        NamedPartSpec::new("upper_mouth", PartPose::offset(0.0, -2.51, 7.0)).cube(
                            cube_grown([54.0, 54.0], [-5.0, -2.0, 0.0], [10.0, 4.0, 4.0], -0.001),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("inner_mouth", PartPose::offset(0.0, -0.51, 7.5))
                            .cube(cube([54.0, 70.0], [-3.0, -2.0, -0.5], [6.0, 4.0, 4.0])),
                    )
                    .child(
                        NamedPartSpec::new("lower_mouth", PartPose::offset(0.0, 1.49, 7.0)).cube(
                            cube_grown([54.0, 62.0], [-5.0, -1.98, 0.0], [10.0, 4.0, 4.0], -0.001),
                        ),
                    ),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_zombie_nautilus_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_zombie_nautilus_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn apply_abstract_boat_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let left_phase_x = ((-state.rowing_time_left).sin() + 1.0) / 2.0;
    let left_x = -std::f32::consts::PI / 3.0
        + left_phase_x.clamp(0.0, 1.0)
            * ((-std::f32::consts::PI / 12.0) - (-std::f32::consts::PI / 3.0));
    let left_phase_y = ((-state.rowing_time_left + 1.0).sin() + 1.0) / 2.0;
    let left_y = -std::f32::consts::PI / 4.0
        + left_phase_y.clamp(0.0, 1.0)
            * (std::f32::consts::PI / 4.0 - (-std::f32::consts::PI / 4.0));
    tree.part_mut_or_panic("left_paddle").pose.rotation.x = left_x;
    tree.part_mut_or_panic("left_paddle").pose.rotation.y = left_y;
    let right_phase_x = ((-state.rowing_time_right).sin() + 1.0) / 2.0;
    let right_x = -std::f32::consts::PI / 3.0
        + right_phase_x.clamp(0.0, 1.0)
            * ((-std::f32::consts::PI / 12.0) - (-std::f32::consts::PI / 3.0));
    let right_phase_y = ((-state.rowing_time_right + 1.0).sin() + 1.0) / 2.0;
    let right_y_base = -std::f32::consts::PI / 4.0
        + right_phase_y.clamp(0.0, 1.0)
            * (std::f32::consts::PI / 4.0 - (-std::f32::consts::PI / 4.0));
    tree.part_mut_or_panic("right_paddle").pose.rotation.x = right_x;
    tree.part_mut_or_panic("right_paddle").pose.rotation.y = std::f32::consts::PI - right_y_base;
}
pub fn apply_acacia_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_acacia_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_acacia_boat_model_tree();
    apply_acacia_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_acacia_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_acacia_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_acacia_chest_boat_model_tree();
    apply_acacia_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_allay_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::offset(0.0, 23.5, 0.0))
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, -3.99, 0.0)).cube(cube(
                    [0.0, 0.0],
                    [-2.5, -5.0, -2.5],
                    [5.0, 5.0, 5.0],
                )),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -4.0, 0.0))
                    .cube(cube([0.0, 10.0], [-1.5, 0.0, -1.0], [3.0, 4.0, 2.0]))
                    .cube(cube_grown(
                        [0.0, 16.0],
                        [-1.5, 0.0, -1.0],
                        [3.0, 5.0, 2.0],
                        -0.2,
                    ))
                    .child(
                        NamedPartSpec::new("right_arm", PartPose::offset(-1.75, 0.5, 0.0)).cube(
                            cube_grown([23.0, 0.0], [-0.75, -0.5, -1.0], [1.0, 4.0, 2.0], -0.01),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("left_arm", PartPose::offset(1.75, 0.5, 0.0)).cube(
                            cube_grown([23.0, 6.0], [-0.25, -0.5, -1.0], [1.0, 4.0, 2.0], -0.01),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("right_wing", PartPose::offset(-0.5, 0.0, 0.6))
                            .cube(cube([16.0, 14.0], [0.0, 1.0, 0.0], [0.0, 5.0, 8.0])),
                    )
                    .child(
                        NamedPartSpec::new("left_wing", PartPose::offset(0.5, 0.0, 0.6))
                            .cube(cube([16.0, 14.0], [0.0, 1.0, 0.0], [0.0, 5.0, 8.0])),
                    ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn apply_allay_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    let flap_speed = state.age_in_ticks * 20.0 * (std::f32::consts::PI / 180.0) + animation_pos;
    let flap_amount = (flap_speed).cos() * std::f32::consts::PI * 0.15 + animation_speed;
    let idle_bob_speed = state.age_in_ticks * 9.0 * (std::f32::consts::PI / 180.0);
    let flying_factor = (animation_speed / 0.3).min(1.0);
    let idle_bob_factor = 1.0 - flying_factor;
    let holding_item_factor = state.holding_animation_progress;
    let dance_speed = if state.is_dancing {
        state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed
    } else {
        0.0
    };
    let dance_frequency = if state.is_dancing {
        (state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 16.0
            * (std::f32::consts::PI / 180.0)
    } else {
        0.0
    };
    let spinning_rotation = if state.is_dancing {
        state.spinning_progress
    } else {
        0.0
    };
    let head_tilt_z = if state.is_dancing {
        (state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 14.0
            * (std::f32::consts::PI / 180.0)
    } else {
        0.0
    };
    let head_tilt_y = if state.is_dancing {
        (state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 30.0
            * (std::f32::consts::PI / 180.0)
    } else {
        0.0
    };
    let arm_flying_rot_x = holding_item_factor
        * (-std::f32::consts::PI / 3.0_f32
            + (flying_factor) * (-1.134464 - -std::f32::consts::PI / 3.0_f32));
    let arm_idle_bob_factor = idle_bob_factor * (1.0 - holding_item_factor);
    let arm_idle_bob_amount = 0.43633232
        - (idle_bob_speed + (std::f32::consts::PI * 3.0 / 2.0)).cos()
            * std::f32::consts::PI
            * 0.075
            * arm_idle_bob_factor;
    let anim_value_0 = if state.is_dancing {
        if state.is_spinning {
            (std::f32::consts::PI * 4.0_f32) * (state.spinning_progress)
        } else {
            tree.part_or_panic("root").pose.rotation.y
        }
    } else {
        tree.part_or_panic("root").pose.rotation.y
    };
    tree.part_mut_or_panic("root").pose.rotation.y = anim_value_0;
    let anim_value_1 = if state.is_dancing {
        ((state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 16.0
            * (std::f32::consts::PI / 180.0))
            * (1.0 - (state.spinning_progress))
    } else {
        tree.part_or_panic("root").pose.rotation.z
    };
    tree.part_mut_or_panic("root").pose.rotation.z = anim_value_1;
    let anim_value_2 = if state.is_dancing {
        ((state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 30.0
            * (std::f32::consts::PI / 180.0))
            * (1.0 - (state.spinning_progress))
    } else {
        tree.part_or_panic("head").pose.rotation.y
    };
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_2;
    let anim_value_3 = if state.is_dancing {
        ((state.age_in_ticks * 8.0 * (std::f32::consts::PI / 180.0) + animation_speed).cos()
            * 14.0
            * (std::f32::consts::PI / 180.0))
            * (1.0 - (state.spinning_progress))
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_3;
    let anim_value_4 = if !(state.is_dancing) {
        state.x_rot_degrees * (std::f32::consts::PI / 180.0)
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_4;
    let anim_value_5 = if !(state.is_dancing) {
        state.y_rot_degrees * (std::f32::consts::PI / 180.0)
    } else {
        tree.part_or_panic("head").pose.rotation.y
    };
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_5;
    let anim_value_6 = 0.43633232 * (1.0 - flying_factor);
    tree.part_mut_or_panic("right_wing").pose.rotation.x = anim_value_6;
    let anim_value_7 = (-std::f32::consts::PI / 4.0_f32) + flap_amount;
    tree.part_mut_or_panic("right_wing").pose.rotation.y = anim_value_7;
    let anim_value_8 = 0.43633232 * (1.0 - flying_factor);
    tree.part_mut_or_panic("left_wing").pose.rotation.x = anim_value_8;
    let anim_value_9 = (std::f32::consts::PI / 4.0_f32) - flap_amount;
    tree.part_mut_or_panic("left_wing").pose.rotation.y = anim_value_9;
    let anim_value_10 = flying_factor * (std::f32::consts::PI / 4.0_f32);
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_10;
    let anim_value_11 =
        tree.part_or_panic("root").pose.offset.y + (idle_bob_speed).cos() * 0.25 * idle_bob_factor;
    tree.part_mut_or_panic("root").pose.offset.y = anim_value_11;
    let anim_value_12 = arm_flying_rot_x;
    tree.part_mut_or_panic("right_arm").pose.rotation.x = anim_value_12;
    let anim_value_13 = arm_flying_rot_x;
    tree.part_mut_or_panic("left_arm").pose.rotation.x = anim_value_13;
    let anim_value_14 = -arm_idle_bob_amount;
    tree.part_mut_or_panic("left_arm").pose.rotation.z = anim_value_14;
    let anim_value_15 = arm_idle_bob_amount;
    tree.part_mut_or_panic("right_arm").pose.rotation.z = anim_value_15;
    let anim_value_16 = 0.27925268 * holding_item_factor;
    tree.part_mut_or_panic("right_arm").pose.rotation.y = anim_value_16;
    let anim_value_17 = -0.27925268 * holding_item_factor;
    tree.part_mut_or_panic("left_arm").pose.rotation.y = anim_value_17;
}
pub fn apply_allay_allay_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_allay_model_setup_anim_ir(tree, state);
}
pub fn build_allay_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_allay_model_tree();
    apply_allay_allay_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn build_axolotl_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("body", PartPose::offset(0.0, 19.5, 5.0))
            .cube(cube([0.0, 11.0], [-4.0, -2.0, -9.0], [8.0, 4.0, 10.0]))
            .cube(cube([2.0, 17.0], [0.0, -3.0, -8.0], [0.0, 5.0, 9.0]))
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 0.0, -9.0))
                    .cube(cube_grown(
                        [0.0, 1.0],
                        [-4.0, -3.0, -5.0],
                        [8.0, 5.0, 5.0],
                        0.001,
                    ))
                    .child(
                        NamedPartSpec::new("top_gills", PartPose::offset(0.0, -3.0, -1.0)).cube(
                            cube_grown([3.0, 37.0], [-4.0, -3.0, 0.0], [8.0, 3.0, 0.0], 0.001),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("left_gills", PartPose::offset(-4.0, 0.0, -1.0)).cube(
                            cube_grown([0.0, 40.0], [-3.0, -5.0, 0.0], [3.0, 7.0, 0.0], 0.001),
                        ),
                    )
                    .child(
                        NamedPartSpec::new("right_gills", PartPose::offset(4.0, 0.0, -1.0)).cube(
                            cube_grown([11.0, 40.0], [0.0, -5.0, 0.0], [3.0, 7.0, 0.0], 0.001),
                        ),
                    ),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.5, 1.0, -1.0)).cube(
                    cube_grown([2.0, 13.0], [-2.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.5, 1.0, -1.0)).cube(
                    cube_grown([2.0, 13.0], [-1.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.5, 1.0, -8.0)).cube(
                    cube_grown([2.0, 13.0], [-2.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(3.5, 1.0, -8.0)).cube(
                    cube_grown([2.0, 13.0], [-1.0, 0.0, 0.0], [3.0, 5.0, 0.0], 0.001),
                ),
            )
            .child(
                NamedPartSpec::new("tail", PartPose::offset(0.0, 0.0, 1.0)).cube(cube(
                    [2.0, 19.0],
                    [0.0, -3.0, 0.0],
                    [0.0, 5.0, 12.0],
                )),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_axolotl_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_axolotl_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_adult_axolotl_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let playing_dead_factor = state.playing_dead_factor;
    let in_water_factor = state.in_water_factor;
    let on_ground_factor = state.on_ground_factor;
    let moving_factor = state.moving_factor;
    let not_moving_factor = 1.0 - moving_factor;
    let mirrored_legs_factor = 1.0 - (on_ground_factor).min(moving_factor);
    let anim_value_0 = tree.part_or_panic("body").pose.rotation.y
        + state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("body").pose.rotation.y = anim_value_0;
    let anim_value_1 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("body").pose.rotation.x
            + (((state.x_rot_degrees) * (std::f32::consts::PI / 180.0)
                + (0.13 * (((state.age_in_ticks) * 0.33).sin())))
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_1;
    let anim_value_2 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("head").pose.rotation.x
            - ((0.13 * (((state.age_in_ticks) * 0.33).sin()))
                * 1.8
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_2;
    let anim_value_3 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("body").pose.offset.y
            - (0.45
                * (((state.age_in_ticks) * 0.33).cos())
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("body").pose.offset.y
    };
    tree.part_mut_or_panic("body").pose.offset.y = anim_value_3;
    let anim_value_4 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("top_gills").pose.rotation.x
            + ((-0.5 * (((state.age_in_ticks) * 0.33).sin()) - 0.8)
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("top_gills").pose.rotation.x
    };
    tree.part_mut_or_panic("top_gills").pose.rotation.x = anim_value_4;
    let anim_value_5 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_gills").pose.rotation.y
            + ((0.3 * (((state.age_in_ticks) * 0.33).sin()) + 0.9)
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("left_gills").pose.rotation.y = anim_value_5;
    let anim_value_6 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_gills").pose.rotation.y
            - ((0.3 * (((state.age_in_ticks) * 0.33).sin()) + 0.9)
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("right_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("right_gills").pose.rotation.y = anim_value_6;
    let anim_value_7 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("tail").pose.rotation.y
            + 0.3
                * (((state.age_in_ticks) * 0.33) * 0.9).cos()
                * ((moving_factor).min(in_water_factor))
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_7;
    let anim_value_8 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
            + (1.8849558 * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_8;
    let anim_value_9 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
            + (-0.4
                * (((state.age_in_ticks) * 0.33).sin())
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.y = anim_value_9;
    let anim_value_10 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
            + ((std::f32::consts::PI / 2.0_f32) * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.z = anim_value_10;
    let anim_value_11 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.x
            + (1.8849558 * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_11;
    let anim_value_12 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.y
            + ((-0.2 * (((state.age_in_ticks) * 0.33).cos()) - 0.1)
                * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.y = anim_value_12;
    let anim_value_13 = if !(((moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.z
            + ((std::f32::consts::PI / 2.0_f32) * ((moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.z = anim_value_13;
    let anim_value_14 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("body").pose.rotation.x
            + ((-0.15 + 0.075 * (((state.age_in_ticks) * 0.075).cos()))
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_14;
    let anim_value_15 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("body").pose.offset.y
            - ((((state.age_in_ticks) * 0.075).sin() * 0.15)
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("body").pose.offset.y
    };
    tree.part_mut_or_panic("body").pose.offset.y = anim_value_15;
    let anim_value_16 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("head").pose.rotation.x
            - ((-0.15 + 0.075 * (((state.age_in_ticks) * 0.075).cos()))
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_16;
    let anim_value_17 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("top_gills").pose.rotation.x
            + (0.2
                * (((state.age_in_ticks) * 0.075).cos())
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("top_gills").pose.rotation.x
    };
    tree.part_mut_or_panic("top_gills").pose.rotation.x = anim_value_17;
    let anim_value_18 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_gills").pose.rotation.y
            + ((-0.3 * (((state.age_in_ticks) * 0.075).cos()) - 0.19)
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("left_gills").pose.rotation.y = anim_value_18;
    let anim_value_19 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_gills").pose.rotation.y
            - ((-0.3 * (((state.age_in_ticks) * 0.075).cos()) - 0.19)
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("right_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("right_gills").pose.rotation.y = anim_value_19;
    let anim_value_20 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
            + (((std::f32::consts::PI * 3.0 / 4.0) - (((state.age_in_ticks) * 0.075).cos()) * 0.11)
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_20;
    let anim_value_21 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
            + (0.47123894 * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.y = anim_value_21;
    let anim_value_22 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
            + (1.7278761 * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.z = anim_value_22;
    let anim_value_23 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.x
            + (((std::f32::consts::PI / 4.0_f32) - (((state.age_in_ticks) * 0.075).cos()) * 0.2)
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_23;
    let anim_value_24 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.y
            + (2.042035 * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.y = anim_value_24;
    let anim_value_25 = if !(((not_moving_factor).min(in_water_factor)) <= 1.0E-5) {
        tree.part_or_panic("tail").pose.rotation.y
            + (0.5
                * (((state.age_in_ticks) * 0.075).cos())
                * ((not_moving_factor).min(in_water_factor)))
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_25;
    let anim_value_26 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("head").pose.rotation.y
            + (0.09
                * (((state.age_in_ticks) * 0.11).cos())
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("head").pose.rotation.y
    };
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_26;
    let anim_value_27 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("tail").pose.rotation.y
            + (0.09
                * (((state.age_in_ticks) * 0.11).cos())
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_27;
    let anim_value_28 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("top_gills").pose.rotation.x
            + ((0.6
                - 0.08
                    * ((((state.age_in_ticks) * 0.11).cos())
                        * (((state.age_in_ticks) * 0.11).cos())
                        + 2.0 * ((state.age_in_ticks) * 0.11).sin()))
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("top_gills").pose.rotation.x
    };
    tree.part_mut_or_panic("top_gills").pose.rotation.x = anim_value_28;
    let anim_value_29 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_gills").pose.rotation.y
            - ((0.6
                - 0.08
                    * ((((state.age_in_ticks) * 0.11).cos())
                        * (((state.age_in_ticks) * 0.11).cos())
                        + 2.0 * ((state.age_in_ticks) * 0.11).sin()))
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("left_gills").pose.rotation.y = anim_value_29;
    let anim_value_30 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_gills").pose.rotation.y
            + ((0.6
                - 0.08
                    * ((((state.age_in_ticks) * 0.11).cos())
                        * (((state.age_in_ticks) * 0.11).cos())
                        + 2.0 * ((state.age_in_ticks) * 0.11).sin()))
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("right_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("right_gills").pose.rotation.y = anim_value_30;
    let anim_value_31 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
            + (0.9424779 * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_31;
    let anim_value_32 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
            + ((1.5
                - (((((state.age_in_ticks) * 0.11).cos())
                    * (((state.age_in_ticks) * 0.11).cos())
                    - 2.0 * (((state.age_in_ticks) * 0.11).cos()))
                    / 5.0))
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.y = anim_value_32;
    let anim_value_33 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
            + (-0.1 * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.z = anim_value_33;
    let anim_value_34 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.x
            + (1.0995574 * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_34;
    let anim_value_35 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.y
            + (((std::f32::consts::PI / 2.0_f32) - (0.7 * (((state.age_in_ticks) * 0.11).cos())))
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.y = anim_value_35;
    let anim_value_36 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
            + (0.9424779 * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_36;
    let anim_value_37 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_hind_leg").pose.rotation.y
            + ((-1.0
                - (((((state.age_in_ticks) * 0.11).cos())
                    * (((state.age_in_ticks) * 0.11).cos())
                    - 2.0 * (((state.age_in_ticks) * 0.11).cos()))
                    / 5.0))
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.y = anim_value_37;
    let anim_value_38 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_front_leg").pose.rotation.x
            + (1.0995574 * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_38;
    let anim_value_39 = if !(((moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_front_leg").pose.rotation.y
            + (((-std::f32::consts::PI / 2.0_f32) - (0.7 * (((state.age_in_ticks) * 0.11).cos())))
                * ((moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.y = anim_value_39;
    let anim_value_40 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("head").pose.rotation.x
            + (-0.09
                * ((((state.age_in_ticks) * 0.09).sin()) * (((state.age_in_ticks) * 0.09).sin())
                    - 2.0 * (((state.age_in_ticks) * 0.09).sin()))
                * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_40;
    let anim_value_41 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("head").pose.rotation.z
            + (-0.2 * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_41;
    let anim_value_42 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("tail").pose.rotation.y
            + ((-0.1
                + 0.1
                    * ((((state.age_in_ticks) * 0.09).sin())
                        * (((state.age_in_ticks) * 0.09).sin())
                        - 2.0 * (((state.age_in_ticks) * 0.09).sin())))
                * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_42;
    let anim_value_43 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("top_gills").pose.rotation.x
            + ((0.6
                + 0.05
                    * ((((state.age_in_ticks) * 0.09).cos())
                        * (((state.age_in_ticks) * 0.09).cos())
                        - 3.0 * (((state.age_in_ticks) * 0.09).sin())))
                * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("top_gills").pose.rotation.x
    };
    tree.part_mut_or_panic("top_gills").pose.rotation.x = anim_value_43;
    let anim_value_44 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_gills").pose.rotation.y
            - ((0.6
                + 0.05
                    * ((((state.age_in_ticks) * 0.09).cos())
                        * (((state.age_in_ticks) * 0.09).cos())
                        - 3.0 * (((state.age_in_ticks) * 0.09).sin())))
                * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("left_gills").pose.rotation.y = anim_value_44;
    let anim_value_45 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("right_gills").pose.rotation.y
            + ((0.6
                + 0.05
                    * ((((state.age_in_ticks) * 0.09).cos())
                        * (((state.age_in_ticks) * 0.09).cos())
                        - 3.0 * (((state.age_in_ticks) * 0.09).sin())))
                * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("right_gills").pose.rotation.y
    };
    tree.part_mut_or_panic("right_gills").pose.rotation.y = anim_value_45;
    let anim_value_46 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
            + (1.1 * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_46;
    let anim_value_47 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
            + (1.0 * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.y = anim_value_47;
    let anim_value_48 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.x
            + (0.8 * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_48;
    let anim_value_49 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.y
            + (2.3 * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.y = anim_value_49;
    let anim_value_50 = if !(((not_moving_factor).min(on_ground_factor)) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.z
            - (0.5 * ((not_moving_factor).min(on_ground_factor)))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.z = anim_value_50;
    let anim_value_51 = if !((playing_dead_factor) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.x + (1.4137167 * (playing_dead_factor))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_51;
    let anim_value_52 = if !((playing_dead_factor) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.y + (1.0995574 * (playing_dead_factor))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.y = anim_value_52;
    let anim_value_53 = if !((playing_dead_factor) <= 1.0E-5) {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
            + ((std::f32::consts::PI / 4.0_f32) * (playing_dead_factor))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.z = anim_value_53;
    let anim_value_54 = if !((playing_dead_factor) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.x
            + ((std::f32::consts::PI / 4.0_f32) * (playing_dead_factor))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_54;
    let anim_value_55 = if !((playing_dead_factor) <= 1.0E-5) {
        tree.part_or_panic("left_front_leg").pose.rotation.y + (2.042035 * (playing_dead_factor))
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.y = anim_value_55;
    let anim_value_56 = if !((playing_dead_factor) <= 1.0E-5) {
        tree.part_or_panic("body").pose.rotation.x + (-0.15 * (playing_dead_factor))
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_56;
    let anim_value_57 = if !((playing_dead_factor) <= 1.0E-5) {
        tree.part_or_panic("body").pose.rotation.z + (0.35 * (playing_dead_factor))
    } else {
        tree.part_or_panic("body").pose.rotation.z
    };
    tree.part_mut_or_panic("body").pose.rotation.z = anim_value_57;
    let anim_value_58 = if !((mirrored_legs_factor) <= 1.0E-5) {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
            + tree.part_or_panic("left_hind_leg").pose.rotation.x * (mirrored_legs_factor)
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_58;
    let anim_value_59 = if !((mirrored_legs_factor) <= 1.0E-5) {
        tree.part_or_panic("right_hind_leg").pose.rotation.y
            + -tree.part_or_panic("left_hind_leg").pose.rotation.y * (mirrored_legs_factor)
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.y = anim_value_59;
    let anim_value_60 = if !((mirrored_legs_factor) <= 1.0E-5) {
        tree.part_or_panic("right_hind_leg").pose.rotation.z
            + -tree.part_or_panic("left_hind_leg").pose.rotation.z * (mirrored_legs_factor)
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.z = anim_value_60;
    let anim_value_61 = if !((mirrored_legs_factor) <= 1.0E-5) {
        tree.part_or_panic("right_front_leg").pose.rotation.x
            + tree.part_or_panic("left_front_leg").pose.rotation.x * (mirrored_legs_factor)
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_61;
    let anim_value_62 = if !((mirrored_legs_factor) <= 1.0E-5) {
        tree.part_or_panic("right_front_leg").pose.rotation.y
            + -tree.part_or_panic("left_front_leg").pose.rotation.y * (mirrored_legs_factor)
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.y = anim_value_62;
    let anim_value_63 = if !((mirrored_legs_factor) <= 1.0E-5) {
        tree.part_or_panic("right_front_leg").pose.rotation.z
            + -tree.part_or_panic("left_front_leg").pose.rotation.z * (mirrored_legs_factor)
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.z = anim_value_63;
}
pub fn apply_axolotl_adult_axolotl_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_adult_axolotl_model_setup_anim_ir(tree, state);
}
pub fn build_axolotl_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_axolotl_model_tree();
    apply_axolotl_adult_axolotl_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_bamboo_chest_raft_raft_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_bamboo_chest_raft_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_bamboo_chest_raft_model_tree();
    apply_bamboo_chest_raft_raft_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn apply_bamboo_raft_raft_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_bamboo_raft_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_bamboo_raft_model_tree();
    apply_bamboo_raft_raft_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_bee_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let speed = if !state.is_on_ground {
        state.age_in_ticks * 120.32113 * (std::f32::consts::PI / 180.0)
    } else {
        0.0
    };
    let speed = if !state.is_angry && !state.is_on_ground {
        (state.age_in_ticks * 0.18).cos()
    } else {
        0.0
    };
    let roll_amount = state.roll_amount;
    let anim_value_0 = state.has_stinger;
    tree.part_mut_or_panic("stinger").visible = anim_value_0;
    let anim_value_1 = if !state.is_on_ground {
        0.0
    } else {
        tree.part_or_panic("right_wing").pose.rotation.y
    };
    tree.part_mut_or_panic("right_wing").pose.rotation.y = anim_value_1;
    let anim_value_2 = if !state.is_on_ground {
        (state.age_in_ticks * 120.32113 * (std::f32::consts::PI / 180.0)).cos()
            * std::f32::consts::PI
            * 0.15
    } else {
        tree.part_or_panic("right_wing").pose.rotation.z
    };
    tree.part_mut_or_panic("right_wing").pose.rotation.z = anim_value_2;
    let anim_value_3 = if !state.is_on_ground {
        tree.part_or_panic("right_wing").pose.rotation.x
    } else {
        tree.part_or_panic("left_wing").pose.rotation.x
    };
    tree.part_mut_or_panic("left_wing").pose.rotation.x = anim_value_3;
    let anim_value_4 = if !state.is_on_ground {
        tree.part_or_panic("right_wing").pose.rotation.y
    } else {
        tree.part_or_panic("left_wing").pose.rotation.y
    };
    tree.part_mut_or_panic("left_wing").pose.rotation.y = anim_value_4;
    let anim_value_5 = if !state.is_on_ground {
        -tree.part_or_panic("right_wing").pose.rotation.z
    } else {
        tree.part_or_panic("left_wing").pose.rotation.z
    };
    tree.part_mut_or_panic("left_wing").pose.rotation.z = anim_value_5;
    let anim_value_6 = if !state.is_on_ground {
        std::f32::consts::PI / 4.0_f32
    } else {
        tree.part_or_panic("front_legs").pose.rotation.x
    };
    tree.part_mut_or_panic("front_legs").pose.rotation.x = anim_value_6;
    let anim_value_7 = if !state.is_on_ground {
        std::f32::consts::PI / 4.0_f32
    } else {
        tree.part_or_panic("middle_legs").pose.rotation.x
    };
    tree.part_mut_or_panic("middle_legs").pose.rotation.x = anim_value_7;
    let anim_value_8 = if !state.is_on_ground {
        std::f32::consts::PI / 4.0_f32
    } else {
        tree.part_or_panic("back_legs").pose.rotation.x
    };
    tree.part_mut_or_panic("back_legs").pose.rotation.x = anim_value_8;
    let anim_value_9 = if !state.is_angry && !state.is_on_ground {
        0.1 + ((state.age_in_ticks * 0.18).cos()) * std::f32::consts::PI * 0.025
    } else {
        tree.part_or_panic("bone").pose.rotation.x
    };
    tree.part_mut_or_panic("bone").pose.rotation.x = anim_value_9;
    let anim_value_10 = if !state.is_angry && !state.is_on_ground {
        tree.part_or_panic("bone").pose.offset.y - (((state.age_in_ticks) * 0.18).cos() * 0.9)
    } else {
        tree.part_or_panic("bone").pose.offset.y
    };
    tree.part_mut_or_panic("bone").pose.offset.y = anim_value_10;
    let anim_value_11 = if !state.is_angry && !state.is_on_ground {
        -((state.age_in_ticks * 0.18).cos()) * std::f32::consts::PI * 0.1
            + (std::f32::consts::PI / 8.0_f32)
    } else {
        tree.part_or_panic("front_legs").pose.rotation.x
    };
    tree.part_mut_or_panic("front_legs").pose.rotation.x = anim_value_11;
    let anim_value_12 = if !state.is_angry && !state.is_on_ground {
        -((state.age_in_ticks * 0.18).cos()) * std::f32::consts::PI * 0.05
            + (std::f32::consts::PI / 4.0_f32)
    } else {
        tree.part_or_panic("back_legs").pose.rotation.x
    };
    tree.part_mut_or_panic("back_legs").pose.rotation.x = anim_value_12;
    let anim_value_13 = if roll_amount > 0.0 {
        tree.part_or_panic("bone").pose.rotation.x
            + (roll_amount)
                * ((((3.0915928) - (tree.part_or_panic("bone").pose.rotation.x))
                    + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI)
    } else {
        tree.part_or_panic("bone").pose.rotation.x
    };
    tree.part_mut_or_panic("bone").pose.rotation.x = anim_value_13;
}
pub fn apply_bee_adult_bee_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_bee_model_setup_anim_ir(tree, state);
}
pub fn build_bee_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_bee_model_tree();
    apply_bee_adult_bee_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_birch_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_birch_boat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_birch_boat_model_tree();
    apply_birch_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_birch_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_birch_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_birch_chest_boat_model_tree();
    apply_birch_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_cat_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new(
        "root",
        PartPose {
            offset: Vec3::new(0.0, 4.8032, 0.0),
            rotation: Vec3::splat(0.0),
            scale: Vec3::splat(0.8),
        },
    )
    .child(
        NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -9.0))
            .cube(cube([0.0, 0.0], [-2.5, -2.0, -3.0], [5.0, 4.0, 5.0]))
            .cube(cube([0.0, 24.0], [-1.5, -0.001, -4.0], [3.0, 2.0, 2.0]))
            .cube(cube([0.0, 10.0], [-2.0, -3.0, 0.0], [1.0, 1.0, 2.0]))
            .cube(cube([6.0, 10.0], [1.0, -3.0, 0.0], [1.0, 1.0, 2.0])),
    )
    .child(
        NamedPartSpec::new(
            "body",
            PartPose::offset_and_rotation(0.0, 12.0, -10.0, 1.570796, 0.0, 0.0),
        )
        .cube(cube([20.0, 0.0], [-2.0, 3.0, -8.0], [4.0, 16.0, 6.0])),
    )
    .child(
        NamedPartSpec::new(
            "tail1",
            PartPose::offset_and_rotation(0.0, 15.0, 8.0, 0.9, 0.0, 0.0),
        )
        .cube(cube([0.0, 15.0], [-0.5, 0.0, 0.0], [1.0, 8.0, 1.0])),
    )
    .child(
        NamedPartSpec::new("tail2", PartPose::offset(0.0, 20.0, 14.0)).cube(cube_grown(
            [4.0, 15.0],
            [-0.5, 0.0, 0.0],
            [1.0, 8.0, 1.0],
            -0.02,
        )),
    )
    .child(
        NamedPartSpec::new("left_hind_leg", PartPose::offset(1.1, 18.0, 5.0)).cube(cube(
            [8.0, 13.0],
            [-1.0, 0.0, 1.0],
            [2.0, 6.0, 2.0],
        )),
    )
    .child(
        NamedPartSpec::new("right_hind_leg", PartPose::offset(-1.1, 18.0, 5.0)).cube(cube(
            [8.0, 13.0],
            [-1.0, 0.0, 1.0],
            [2.0, 6.0, 2.0],
        )),
    )
    .child(
        NamedPartSpec::new("left_front_leg", PartPose::offset(1.2, 14.1, -5.0)).cube(cube(
            [40.0, 0.0],
            [-1.0, 0.0, 0.0],
            [2.0, 10.0, 2.0],
        )),
    )
    .child(
        NamedPartSpec::new("right_front_leg", PartPose::offset(-1.2, 14.1, -5.0)).cube(cube(
            [40.0, 0.0],
            [-1.0, 0.0, 0.0],
            [2.0, 10.0, 2.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_cat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_cat_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_adult_feline_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let age_scale = state.age_scale;
    let animation_speed = if !state.is_sitting {
        state.walk_animation_speed
    } else {
        0.0
    };
    let animation_pos = if !state.is_sitting {
        state.walk_animation_pos
    } else {
        0.0
    };
    let anim_value_0 = if state.is_crouching {
        tree.part_or_panic("body").pose.offset.y + (1.0 * age_scale)
    } else {
        tree.part_or_panic("body").pose.offset.y
    };
    tree.part_mut_or_panic("body").pose.offset.y = anim_value_0;
    let anim_value_1 = if state.is_crouching {
        tree.part_or_panic("head").pose.offset.y + (2.0 * age_scale)
    } else {
        tree.part_or_panic("head").pose.offset.y
    };
    tree.part_mut_or_panic("head").pose.offset.y = anim_value_1;
    let anim_value_2 = if state.is_crouching {
        tree.part_or_panic("tail1").pose.offset.y + (1.0 * age_scale)
    } else {
        tree.part_or_panic("tail1").pose.offset.y
    };
    tree.part_mut_or_panic("tail1").pose.offset.y = anim_value_2;
    let anim_value_3 = if state.is_crouching {
        tree.part_or_panic("tail2").pose.offset.y + (-4.0 * age_scale)
    } else {
        tree.part_or_panic("tail2").pose.offset.y
    };
    tree.part_mut_or_panic("tail2").pose.offset.y = anim_value_3;
    let anim_value_4 = if state.is_crouching {
        tree.part_or_panic("tail2").pose.offset.z + (2.0 * age_scale)
    } else {
        tree.part_or_panic("tail2").pose.offset.z
    };
    tree.part_mut_or_panic("tail2").pose.offset.z = anim_value_4;
    let anim_value_5 = if state.is_crouching {
        std::f32::consts::PI / 2.0_f32
    } else {
        tree.part_or_panic("tail1").pose.rotation.x
    };
    tree.part_mut_or_panic("tail1").pose.rotation.x = anim_value_5;
    let anim_value_6 = if state.is_crouching {
        std::f32::consts::PI / 2.0_f32
    } else {
        tree.part_or_panic("tail2").pose.rotation.x
    };
    tree.part_mut_or_panic("tail2").pose.rotation.x = anim_value_6;
    let anim_value_7 = if !(state.is_crouching) && (state.is_sprinting) {
        tree.part_or_panic("tail1").pose.offset.y
    } else {
        tree.part_or_panic("tail2").pose.offset.y
    };
    tree.part_mut_or_panic("tail2").pose.offset.y = anim_value_7;
    let anim_value_8 = if !(state.is_crouching) && (state.is_sprinting) {
        tree.part_or_panic("tail2").pose.offset.z + (2.0 * age_scale)
    } else {
        tree.part_or_panic("tail2").pose.offset.z
    };
    tree.part_mut_or_panic("tail2").pose.offset.z = anim_value_8;
    let anim_value_9 = if !(state.is_crouching) && (state.is_sprinting) {
        std::f32::consts::PI / 2.0_f32
    } else {
        tree.part_or_panic("tail1").pose.rotation.x
    };
    tree.part_mut_or_panic("tail1").pose.rotation.x = anim_value_9;
    let anim_value_10 = if !(state.is_crouching) && (state.is_sprinting) {
        std::f32::consts::PI / 2.0_f32
    } else {
        tree.part_or_panic("tail2").pose.rotation.x
    };
    tree.part_mut_or_panic("tail2").pose.rotation.x = anim_value_10;
    let anim_value_11 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_11;
    let anim_value_12 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_12;
    let anim_value_13 = if !state.is_sitting {
        std::f32::consts::PI / 2.0_f32
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_13;
    let anim_value_14 = if (!state.is_sitting) && (state.is_sprinting) {
        ((state.walk_animation_pos) * 0.6662).cos() * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_14;
    let anim_value_15 = if (!state.is_sitting) && (state.is_sprinting) {
        ((state.walk_animation_pos) * 0.6662 + 0.3).cos() * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_15;
    let anim_value_16 = if (!state.is_sitting) && (state.is_sprinting) {
        ((state.walk_animation_pos) * 0.6662 + std::f32::consts::PI + 0.3).cos()
            * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_16;
    let anim_value_17 = if (!state.is_sitting) && (state.is_sprinting) {
        ((state.walk_animation_pos) * 0.6662 + std::f32::consts::PI).cos()
            * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_17;
    let anim_value_18 = if (!state.is_sitting) && (state.is_sprinting) {
        1.7278761
            + (std::f32::consts::PI / 10.0_f32)
                * (state.walk_animation_pos).cos()
                * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("tail2").pose.rotation.x
    };
    tree.part_mut_or_panic("tail2").pose.rotation.x = anim_value_18;
    let anim_value_19 = if (!state.is_sitting) && (!(state.is_sprinting)) {
        ((state.walk_animation_pos) * 0.6662).cos() * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_19;
    let anim_value_20 = if (!state.is_sitting) && (!(state.is_sprinting)) {
        ((state.walk_animation_pos) * 0.6662 + std::f32::consts::PI).cos()
            * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_20;
    let anim_value_21 = if (!state.is_sitting) && (!(state.is_sprinting)) {
        ((state.walk_animation_pos) * 0.6662 + std::f32::consts::PI).cos()
            * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_21;
    let anim_value_22 = if (!state.is_sitting) && (!(state.is_sprinting)) {
        ((state.walk_animation_pos) * 0.6662).cos() * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_22;
    let anim_value_23 = if ((!state.is_sitting) && (!(state.is_sprinting))) && (!state.is_crouching)
    {
        1.7278761
            + (std::f32::consts::PI / 4.0_f32)
                * (state.walk_animation_pos).cos()
                * (state.walk_animation_speed)
    } else {
        tree.part_or_panic("tail2").pose.rotation.x
    };
    tree.part_mut_or_panic("tail2").pose.rotation.x = anim_value_23;
    let anim_value_24 =
        if ((!state.is_sitting) && (!(state.is_sprinting))) && (!(!state.is_crouching)) {
            1.7278761 + 0.47123894 * (state.walk_animation_pos).cos() * (state.walk_animation_speed)
        } else {
            tree.part_or_panic("tail2").pose.rotation.x
        };
    tree.part_mut_or_panic("tail2").pose.rotation.x = anim_value_24;
    let anim_value_25 = if state.is_sitting {
        std::f32::consts::PI / 4.0_f32
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_25;
    let anim_value_26 = if state.is_sitting {
        tree.part_or_panic("body").pose.offset.y + (-4.0 * age_scale)
    } else {
        tree.part_or_panic("body").pose.offset.y
    };
    tree.part_mut_or_panic("body").pose.offset.y = anim_value_26;
    let anim_value_27 = if state.is_sitting {
        tree.part_or_panic("body").pose.offset.z + (5.0 * age_scale)
    } else {
        tree.part_or_panic("body").pose.offset.z
    };
    tree.part_mut_or_panic("body").pose.offset.z = anim_value_27;
    let anim_value_28 = if state.is_sitting {
        tree.part_or_panic("head").pose.offset.y + (-3.3 * age_scale)
    } else {
        tree.part_or_panic("head").pose.offset.y
    };
    tree.part_mut_or_panic("head").pose.offset.y = anim_value_28;
    let anim_value_29 = if state.is_sitting {
        tree.part_or_panic("head").pose.offset.z + (1.0 * age_scale)
    } else {
        tree.part_or_panic("head").pose.offset.z
    };
    tree.part_mut_or_panic("head").pose.offset.z = anim_value_29;
    let anim_value_30 = if state.is_sitting {
        tree.part_or_panic("tail1").pose.offset.y + (8.0 * age_scale)
    } else {
        tree.part_or_panic("tail1").pose.offset.y
    };
    tree.part_mut_or_panic("tail1").pose.offset.y = anim_value_30;
    let anim_value_31 = if state.is_sitting {
        tree.part_or_panic("tail1").pose.offset.z + (-2.0 * age_scale)
    } else {
        tree.part_or_panic("tail1").pose.offset.z
    };
    tree.part_mut_or_panic("tail1").pose.offset.z = anim_value_31;
    let anim_value_32 = if state.is_sitting {
        tree.part_or_panic("tail2").pose.offset.y + (2.0 * age_scale)
    } else {
        tree.part_or_panic("tail2").pose.offset.y
    };
    tree.part_mut_or_panic("tail2").pose.offset.y = anim_value_32;
    let anim_value_33 = if state.is_sitting {
        tree.part_or_panic("tail2").pose.offset.z + (-0.8 * age_scale)
    } else {
        tree.part_or_panic("tail2").pose.offset.z
    };
    tree.part_mut_or_panic("tail2").pose.offset.z = anim_value_33;
    let anim_value_34 = if state.is_sitting {
        1.7278761
    } else {
        tree.part_or_panic("tail1").pose.rotation.x
    };
    tree.part_mut_or_panic("tail1").pose.rotation.x = anim_value_34;
    let anim_value_35 = if state.is_sitting {
        2.670354
    } else {
        tree.part_or_panic("tail2").pose.rotation.x
    };
    tree.part_mut_or_panic("tail2").pose.rotation.x = anim_value_35;
    let anim_value_36 = if state.is_sitting {
        -std::f32::consts::PI / 20.0_f32
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_36;
    let anim_value_37 = if state.is_sitting {
        tree.part_or_panic("left_front_leg").pose.offset.y + (2.0 * age_scale)
    } else {
        tree.part_or_panic("left_front_leg").pose.offset.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.offset.y = anim_value_37;
    let anim_value_38 = if state.is_sitting {
        tree.part_or_panic("left_front_leg").pose.offset.z - (2.0 * age_scale)
    } else {
        tree.part_or_panic("left_front_leg").pose.offset.z
    };
    tree.part_mut_or_panic("left_front_leg").pose.offset.z = anim_value_38;
    let anim_value_39 = if state.is_sitting {
        -std::f32::consts::PI / 20.0_f32
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_39;
    let anim_value_40 = if state.is_sitting {
        tree.part_or_panic("right_front_leg").pose.offset.y + (2.0 * age_scale)
    } else {
        tree.part_or_panic("right_front_leg").pose.offset.y
    };
    tree.part_mut_or_panic("right_front_leg").pose.offset.y = anim_value_40;
    let anim_value_41 = if state.is_sitting {
        tree.part_or_panic("right_front_leg").pose.offset.z - (2.0 * age_scale)
    } else {
        tree.part_or_panic("right_front_leg").pose.offset.z
    };
    tree.part_mut_or_panic("right_front_leg").pose.offset.z = anim_value_41;
    let anim_value_42 = if state.is_sitting {
        -std::f32::consts::PI / 2.0_f32
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_42;
    let anim_value_43 = if state.is_sitting {
        tree.part_or_panic("left_hind_leg").pose.offset.y + (3.0 * age_scale)
    } else {
        tree.part_or_panic("left_hind_leg").pose.offset.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.offset.y = anim_value_43;
    let anim_value_44 = if state.is_sitting {
        tree.part_or_panic("left_hind_leg").pose.offset.z - (4.0 * age_scale)
    } else {
        tree.part_or_panic("left_hind_leg").pose.offset.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.offset.z = anim_value_44;
    let anim_value_45 = if state.is_sitting {
        -std::f32::consts::PI / 2.0_f32
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_45;
    let anim_value_46 = if state.is_sitting {
        tree.part_or_panic("right_hind_leg").pose.offset.y + (3.0 * age_scale)
    } else {
        tree.part_or_panic("right_hind_leg").pose.offset.y
    };
    tree.part_mut_or_panic("right_hind_leg").pose.offset.y = anim_value_46;
    let anim_value_47 = if state.is_sitting {
        tree.part_or_panic("right_hind_leg").pose.offset.z - (4.0 * age_scale)
    } else {
        tree.part_or_panic("right_hind_leg").pose.offset.z
    };
    tree.part_mut_or_panic("right_hind_leg").pose.offset.z = anim_value_47;
    let anim_value_48 = if state.lie_down_amount > 0.0 {
        tree.part_or_panic("head").pose.rotation.z
            + (state.lie_down_amount)
                * ((((-1.2707963) - (tree.part_or_panic("head").pose.rotation.z)) + 180.0)
                    .rem_euclid(360.0)
                    - 180.0)
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_48;
    let anim_value_49 = if state.lie_down_amount > 0.0 {
        tree.part_or_panic("head").pose.rotation.y
            + (state.lie_down_amount)
                * ((((1.2707963) - (tree.part_or_panic("head").pose.rotation.y)) + 180.0)
                    .rem_euclid(360.0)
                    - 180.0)
    } else {
        tree.part_or_panic("head").pose.rotation.y
    };
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_49;
    let anim_value_50 = if state.lie_down_amount > 0.0 {
        -1.2707963
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_50;
    let anim_value_51 = if state.lie_down_amount > 0.0 {
        -0.47079635
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_51;
    let anim_value_52 = if state.lie_down_amount > 0.0 {
        -0.2
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.z = anim_value_52;
    let anim_value_53 = if state.lie_down_amount > 0.0 {
        tree.part_or_panic("right_front_leg").pose.offset.x + (age_scale)
    } else {
        tree.part_or_panic("right_front_leg").pose.offset.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.offset.x = anim_value_53;
    let anim_value_54 = if state.lie_down_amount > 0.0 {
        -0.4
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_54;
    let anim_value_55 = if state.lie_down_amount > 0.0 {
        0.5
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_55;
    let anim_value_56 = if state.lie_down_amount > 0.0 {
        -0.5
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.z = anim_value_56;
    let anim_value_57 = if state.lie_down_amount > 0.0 {
        tree.part_or_panic("right_hind_leg").pose.offset.x + (0.8 * age_scale)
    } else {
        tree.part_or_panic("right_hind_leg").pose.offset.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.offset.x = anim_value_57;
    let anim_value_58 = if state.lie_down_amount > 0.0 {
        tree.part_or_panic("right_hind_leg").pose.offset.y + (2.0 * age_scale)
    } else {
        tree.part_or_panic("right_hind_leg").pose.offset.y
    };
    tree.part_mut_or_panic("right_hind_leg").pose.offset.y = anim_value_58;
    let anim_value_59 = if state.lie_down_amount > 0.0 {
        tree.part_or_panic("tail1").pose.rotation.x
            + (state.lie_down_amount_tail)
                * ((((0.8) - (tree.part_or_panic("tail1").pose.rotation.x)) + 180.0)
                    .rem_euclid(360.0)
                    - 180.0)
    } else {
        tree.part_or_panic("tail1").pose.rotation.x
    };
    tree.part_mut_or_panic("tail1").pose.rotation.x = anim_value_59;
    let anim_value_60 = if state.lie_down_amount > 0.0 {
        tree.part_or_panic("tail2").pose.rotation.x
            + (state.lie_down_amount_tail)
                * ((((-0.4) - (tree.part_or_panic("tail2").pose.rotation.x)) + 180.0)
                    .rem_euclid(360.0)
                    - 180.0)
    } else {
        tree.part_or_panic("tail2").pose.rotation.x
    };
    tree.part_mut_or_panic("tail2").pose.rotation.x = anim_value_60;
    let anim_value_61 = if state.relax_state_one_amount > 0.0 {
        tree.part_or_panic("head").pose.rotation.x
            + (state.relax_state_one_amount)
                * ((((-0.58177644) - (tree.part_or_panic("head").pose.rotation.x)) + 180.0)
                    .rem_euclid(360.0)
                    - 180.0)
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_61;
}
pub fn apply_cat_adult_feline_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_adult_feline_model_setup_anim_ir(tree, state);
}
pub fn build_cat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_cat_model_tree();
    apply_cat_adult_feline_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_cherry_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_cherry_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_cherry_boat_model_tree();
    apply_cherry_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_cherry_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_cherry_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_cherry_chest_boat_model_tree();
    apply_cherry_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn apply_dark_oak_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_dark_oak_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_dark_oak_boat_model_tree();
    apply_dark_oak_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_dark_oak_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_dark_oak_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_dark_oak_chest_boat_model_tree();
    apply_dark_oak_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_dolphin_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("body", PartPose::offset(0.0, 22.0, -5.0))
            .cube(cube([22.0, 0.0], [-4.0, -7.0, 0.0], [8.0, 7.0, 13.0]))
            .child(
                NamedPartSpec::new(
                    "back_fin",
                    PartPose::offset_and_rotation(0.0, 0.0, 0.0, 1.047198, 0.0, 0.0),
                )
                .cube(cube([51.0, 0.0], [-0.5, 0.0, 8.0], [1.0, 4.0, 5.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_fin",
                    PartPose::offset_and_rotation(2.0, -2.0, 4.0, 1.047198, 0.0, 2.094395),
                )
                .cube(cube_mirrored(
                    [48.0, 20.0],
                    [-0.5, -4.0, 0.0],
                    [1.0, 4.0, 7.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "right_fin",
                    PartPose::offset_and_rotation(-2.0, -2.0, 4.0, 1.047198, 0.0, -2.094395),
                )
                .cube(cube([48.0, 20.0], [-0.5, -4.0, 0.0], [1.0, 4.0, 7.0])),
            )
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(0.0, -2.5, 11.0, -0.10472, 0.0, 0.0),
                )
                .cube(cube([0.0, 19.0], [-2.0, -2.5, 0.0], [4.0, 5.0, 11.0]))
                .child(
                    NamedPartSpec::new("tail_fin", PartPose::offset(0.0, 0.0, 9.0)).cube(cube(
                        [19.0, 20.0],
                        [-5.0, -0.5, 0.0],
                        [10.0, 1.0, 6.0],
                    )),
                ),
            )
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, -4.0, -3.0))
                    .cube(cube([0.0, 0.0], [-4.0, -3.0, -3.0], [8.0, 7.0, 6.0]))
                    .child(NamedPartSpec::new("nose", PartPose::ZERO).cube(cube(
                        [0.0, 13.0],
                        [-1.0, 2.0, -7.0],
                        [2.0, 2.0, 4.0],
                    ))),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_dolphin_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_dolphin_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_dolphin_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_0;
    let anim_value_1 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("body").pose.rotation.y = anim_value_1;
    let anim_value_2 = if state.is_moving {
        tree.part_or_panic("body").pose.rotation.x
            + (-0.05 - 0.05 * (state.age_in_ticks * 0.3).cos())
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_2;
    let anim_value_3 = if state.is_moving {
        -0.1 * (state.age_in_ticks * 0.3).cos()
    } else {
        tree.part_or_panic("tail").pose.rotation.x
    };
    tree.part_mut_or_panic("tail").pose.rotation.x = anim_value_3;
    let anim_value_4 = if state.is_moving {
        -0.2 * (state.age_in_ticks * 0.3).cos()
    } else {
        tree.part_or_panic("tail_fin").pose.rotation.x
    };
    tree.part_mut_or_panic("tail_fin").pose.rotation.x = anim_value_4;
}
pub fn apply_dolphin_dolphin_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_dolphin_model_setup_anim_ir(tree, state);
}
pub fn build_dolphin_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_dolphin_model_tree();
    apply_dolphin_dolphin_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_evoker_fangs_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new("base", PartPose::offset(-5.0, 24.0, -5.0))
            .cube(cube([0.0, 0.0], [0.0, 0.0, 0.0], [10.0, 12.0, 10.0]))
            .child(
                NamedPartSpec::new(
                    "upper_jaw",
                    PartPose::offset_and_rotation(6.5, 0.0, 1.0, 0.0, 0.0, 2.042035),
                )
                .cube(cube([40.0, 0.0], [0.0, 0.0, 0.0], [4.0, 14.0, 8.0])),
            )
            .child(
                NamedPartSpec::new(
                    "lower_jaw",
                    PartPose::offset_and_rotation(3.5, 0.0, 9.0, 0.0, 3.141593, 4.24115),
                )
                .cube(cube([40.0, 0.0], [0.0, 0.0, 0.0], [4.0, 14.0, 8.0])),
            ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_evoker_fangs_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_evoker_fangs_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_evoker_fangs_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let bite_progress = state.bite_progress;
    let bite_amount = (bite_progress * 2.0).min(1.0);
    let bite_amount = 1.0 - bite_amount * bite_amount * bite_amount;
    let mut pre_scale = 1.0;
    if bite_progress > 0.9 {
        pre_scale *= (1.0 - bite_progress) / 0.1;
    }
    tree.part_mut_or_panic("upper_jaw").pose.rotation.z =
        std::f32::consts::PI - bite_amount * 0.35 * std::f32::consts::PI;
    tree.part_mut_or_panic("lower_jaw").pose.rotation.z =
        std::f32::consts::PI + bite_amount * 0.35 * std::f32::consts::PI;
    let base_y = tree.part_or_panic("base").pose.offset.y
        - (bite_progress + (bite_progress * 2.7).sin()) * 7.2;
    tree.part_mut_or_panic("base").pose.offset.y = base_y;
    tree.part_mut_or_panic("root").pose.offset.y = 24.0 - 20.0 * pre_scale;
    tree.part_mut_or_panic("root").pose.scale.x = pre_scale;
    tree.part_mut_or_panic("root").pose.scale.y = pre_scale;
    tree.part_mut_or_panic("root").pose.scale.z = pre_scale;
}
pub fn apply_evoker_fangs_evoker_fangs_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_evoker_fangs_model_setup_anim_ir(tree, state);
}
pub fn build_evoker_fangs_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_evoker_fangs_model_tree();
    apply_evoker_fangs_evoker_fangs_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_goat_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(1.0, 14.0, 0.0))
                    .cube(cube([2.0, 61.0], [-6.0, -11.0, -10.0], [3.0, 2.0, 1.0]))
                    .cube(cube_mirrored(
                        [2.0, 61.0],
                        [2.0, -11.0, -10.0],
                        [3.0, 2.0, 1.0],
                    ))
                    .cube(cube_mirrored(
                        [23.0, 52.0],
                        [-0.5, -3.0, -14.0],
                        [0.0, 7.0, 5.0],
                    ))
                    .child(
                        NamedPartSpec::new("left_horn", PartPose::offset(0.0, 0.0, 0.0))
                            .cube(cube([12.0, 55.0], [-0.01, -16.0, -10.0], [2.0, 7.0, 2.0])),
                    )
                    .child(
                        NamedPartSpec::new("right_horn", PartPose::offset(0.0, 0.0, 0.0))
                            .cube(cube([12.0, 55.0], [-2.99, -16.0, -10.0], [2.0, 7.0, 2.0])),
                    )
                    .child(
                        NamedPartSpec::new(
                            "nose",
                            PartPose::offset_and_rotation(0.0, -8.0, -8.0, 0.9599, 0.0, 0.0),
                        )
                        .cube(cube(
                            [34.0, 46.0],
                            [-3.0, -4.0, -8.0],
                            [5.0, 7.0, 10.0],
                        )),
                    ),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 24.0, 0.0))
                    .cube(cube([1.0, 1.0], [-4.0, -17.0, -7.0], [9.0, 11.0, 16.0]))
                    .cube(cube([0.0, 28.0], [-5.0, -18.0, -8.0], [11.0, 14.0, 11.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(1.0, 14.0, 4.0)).cube(cube(
                    [36.0, 29.0],
                    [0.0, 4.0, 0.0],
                    [3.0, 6.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 14.0, 4.0)).cube(cube(
                    [49.0, 29.0],
                    [0.0, 4.0, 0.0],
                    [3.0, 6.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(1.0, 14.0, -6.0)).cube(cube(
                    [49.0, 2.0],
                    [0.0, 0.0, 0.0],
                    [3.0, 10.0, 3.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.0, 14.0, -6.0))
                    .cube(cube([35.0, 2.0], [0.0, 0.0, 0.0], [3.0, 10.0, 3.0])),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_goat_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_goat_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_goat_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.has_left_horn;
    tree.part_mut_or_panic("left_horn").visible = anim_value_0;
    let anim_value_1 = state.has_right_horn;
    tree.part_mut_or_panic("right_horn").visible = anim_value_1;
    let anim_value_2 = if state.ramming_x_head_rot != 0.0 {
        state.ramming_x_head_rot
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_2;
}
pub fn apply_goat_goat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
    apply_goat_model_setup_anim_ir(tree, state);
}
pub fn build_goat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_goat_model_tree();
    apply_goat_goat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_hoglin_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 7.0, 0.0))
                    .cube(cube([1.0, 1.0], [-8.0, -7.0, -13.0], [16.0, 14.0, 26.0]))
                    .child(
                        NamedPartSpec::new("mane", PartPose::offset(0.0, -14.0, -7.0)).cube(
                            cube_grown([90.0, 33.0], [0.0, 0.0, -9.0], [0.0, 10.0, 19.0], 0.001),
                        ),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "head",
                    PartPose::offset_and_rotation(0.0, 2.0, -12.0, 0.872665, 0.0, 0.0),
                )
                .cube(cube([61.0, 1.0], [-7.0, -3.0, -19.0], [14.0, 6.0, 19.0]))
                .child(
                    NamedPartSpec::new(
                        "right_ear",
                        PartPose::offset_and_rotation(-6.0, -2.0, -3.0, 0.0, 0.0, -0.698132),
                    )
                    .cube(cube(
                        [1.0, 1.0],
                        [-6.0, -1.0, -2.0],
                        [6.0, 1.0, 4.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_ear",
                        PartPose::offset_and_rotation(6.0, -2.0, -3.0, 0.0, 0.0, 0.698132),
                    )
                    .cube(cube([1.0, 6.0], [0.0, -1.0, -2.0], [6.0, 1.0, 4.0])),
                )
                .child(
                    NamedPartSpec::new("right_horn", PartPose::offset(-7.0, 2.0, -12.0))
                        .cube(cube([10.0, 13.0], [-1.0, -11.0, -1.0], [2.0, 11.0, 2.0])),
                )
                .child(
                    NamedPartSpec::new("left_horn", PartPose::offset(7.0, 2.0, -12.0)).cube(cube(
                        [1.0, 13.0],
                        [-1.0, -11.0, -1.0],
                        [2.0, 11.0, 2.0],
                    )),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 10.0, -8.5))
                    .cube(cube([66.0, 42.0], [-3.0, 0.0, -3.0], [6.0, 14.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 10.0, -8.5)).cube(cube(
                    [41.0, 42.0],
                    [-3.0, 0.0, -3.0],
                    [6.0, 14.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-5.0, 13.0, 10.0))
                    .cube(cube([21.0, 45.0], [-2.5, 0.0, -2.5], [5.0, 11.0, 5.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(5.0, 13.0, 10.0)).cube(cube(
                    [0.0, 45.0],
                    [-2.5, 0.0, -2.5],
                    [5.0, 11.0, 5.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_hoglin_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_hoglin_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_hoglin_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    let headbutt_lerp_factor =
        1.0 - (10.0_f32 - 2.0_f32 * state.attack_animation_remaining_ticks).abs() / 10.0;
    let amplitude_multiplier = 1.2;
    let anim_value_0 =
        (-std::f32::consts::PI * 2.0 / 9.0) - animation_speed * (animation_pos).sin();
    tree.part_mut_or_panic("right_ear").pose.rotation.z = anim_value_0;
    let anim_value_1 = (std::f32::consts::PI * 2.0 / 9.0) + animation_speed * (animation_pos).sin();
    tree.part_mut_or_panic("left_ear").pose.rotation.z = anim_value_1;
    let anim_value_2 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_2;
    let anim_value_3 =
        0.87266463 + (headbutt_lerp_factor) * (-std::f32::consts::PI / 9.0_f32 - 0.87266463);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_3;
    let anim_value_4 = (animation_pos).cos() * 1.2 * animation_speed;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_4;
    let anim_value_5 = (animation_pos + std::f32::consts::PI).cos() * 1.2 * animation_speed;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_5;
    let anim_value_6 = tree.part_or_panic("left_front_leg").pose.rotation.x;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_6;
    let anim_value_7 = tree.part_or_panic("right_front_leg").pose.rotation.x;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_7;
}
pub fn apply_hoglin_hoglin_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_hoglin_model_setup_anim_ir(tree, state);
}
pub fn build_hoglin_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_hoglin_model_tree();
    apply_hoglin_hoglin_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_iron_golem_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, -7.0, -2.0))
                    .cube(cube([0.0, 0.0], [-4.0, -12.0, -5.5], [8.0, 10.0, 8.0]))
                    .cube(cube([24.0, 0.0], [-1.0, -5.0, -7.5], [2.0, 4.0, 2.0])),
            )
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, -7.0, 0.0))
                    .cube(cube([0.0, 40.0], [-9.0, -2.0, -6.0], [18.0, 12.0, 11.0]))
                    .cube(cube_grown(
                        [0.0, 70.0],
                        [-4.5, 10.0, -3.0],
                        [9.0, 5.0, 6.0],
                        0.5,
                    )),
            )
            .child(
                NamedPartSpec::new("right_arm", PartPose::offset(0.0, -7.0, 0.0)).cube(cube(
                    [60.0, 21.0],
                    [-13.0, -2.5, -3.0],
                    [4.0, 30.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_arm", PartPose::offset(0.0, -7.0, 0.0)).cube(cube(
                    [60.0, 58.0],
                    [9.0, -2.5, -3.0],
                    [4.0, 30.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_leg", PartPose::offset(-4.0, 11.0, 0.0)).cube(cube(
                    [37.0, 0.0],
                    [-3.5, -3.0, -3.0],
                    [6.0, 16.0, 5.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_leg", PartPose::offset(5.0, 11.0, 0.0)).cube(
                    cube_mirrored([60.0, 0.0], [-3.5, -3.0, -3.0], [6.0, 16.0, 5.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn apply_iron_golem_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let attack_tick = state.attack_ticks_remaining;
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    let offer_flower_tick = if !(attack_tick > 0.0) {
        state.offer_flower_tick
    } else {
        0.0
    };
    let anim_value_0 = if attack_tick > 0.0 {
        -2.0 + 1.5 * (((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
            / ((10.0) * 0.25)
    } else {
        tree.part_or_panic("right_arm").pose.rotation.x
    };
    tree.part_mut_or_panic("right_arm").pose.rotation.x = anim_value_0;
    let anim_value_1 = if attack_tick > 0.0 {
        -2.0 + 1.5 * (((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
            / ((10.0) * 0.25)
    } else {
        tree.part_or_panic("left_arm").pose.rotation.x
    };
    tree.part_mut_or_panic("left_arm").pose.rotation.x = anim_value_1;
    let anim_value_2 = if (!(attack_tick > 0.0)) && ((state.offer_flower_tick) > 0.0_f32) {
        -0.8 + 0.025 * (((state.offer_flower_tick) % (70.0) - (70.0) * 0.5).abs() - (70.0) * 0.25)
            / ((70.0) * 0.25)
    } else {
        tree.part_or_panic("right_arm").pose.rotation.x
    };
    tree.part_mut_or_panic("right_arm").pose.rotation.x = anim_value_2;
    let anim_value_3 = if (!(attack_tick > 0.0)) && ((state.offer_flower_tick) > 0.0_f32) {
        0.0
    } else {
        tree.part_or_panic("left_arm").pose.rotation.x
    };
    tree.part_mut_or_panic("left_arm").pose.rotation.x = anim_value_3;
    let anim_value_4 = if (!(attack_tick > 0.0)) && (!((state.offer_flower_tick) > 0.0_f32)) {
        (-0.2
            + 1.5 * (((animation_pos) % (13.0) - (13.0) * 0.5).abs() - (13.0) * 0.25)
                / ((13.0) * 0.25))
            * animation_speed
    } else {
        tree.part_or_panic("right_arm").pose.rotation.x
    };
    tree.part_mut_or_panic("right_arm").pose.rotation.x = anim_value_4;
    let anim_value_5 = if (!(attack_tick > 0.0)) && (!((state.offer_flower_tick) > 0.0_f32)) {
        (-0.2
            - 1.5 * (((animation_pos) % (13.0) - (13.0) * 0.5).abs() - (13.0) * 0.25)
                / ((13.0) * 0.25))
            * animation_speed
    } else {
        tree.part_or_panic("left_arm").pose.rotation.x
    };
    tree.part_mut_or_panic("left_arm").pose.rotation.x = anim_value_5;
    let anim_value_6 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_6;
    let anim_value_7 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_7;
    let anim_value_8 = -1.5 * (((animation_pos) % (13.0) - (13.0) * 0.5).abs() - (13.0) * 0.25)
        / ((13.0) * 0.25)
        * animation_speed;
    tree.part_mut_or_panic("right_leg").pose.rotation.x = anim_value_8;
    let anim_value_9 = 1.5 * (((animation_pos) % (13.0) - (13.0) * 0.5).abs() - (13.0) * 0.25)
        / ((13.0) * 0.25)
        * animation_speed;
    tree.part_mut_or_panic("left_leg").pose.rotation.x = anim_value_9;
    let anim_value_10 = 0.0;
    tree.part_mut_or_panic("right_leg").pose.rotation.y = anim_value_10;
    let anim_value_11 = 0.0;
    tree.part_mut_or_panic("left_leg").pose.rotation.y = anim_value_11;
}
pub fn apply_iron_golem_iron_golem_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_iron_golem_model_setup_anim_ir(tree, state);
}
pub fn apply_jungle_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_jungle_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_jungle_boat_model_tree();
    apply_jungle_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_jungle_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_jungle_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_jungle_chest_boat_model_tree();
    apply_jungle_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_llama_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 7.0, -6.0))
                    .cube(cube([0.0, 0.0], [-2.0, -14.0, -10.0], [4.0, 4.0, 9.0]))
                    .cube(cube([0.0, 14.0], [-4.0, -16.0, -6.0], [8.0, 18.0, 6.0]))
                    .cube(cube([17.0, 0.0], [-4.0, -19.0, -4.0], [3.0, 3.0, 2.0]))
                    .cube(cube([17.0, 0.0], [1.0, -19.0, -4.0], [3.0, 3.0, 2.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 5.0, 2.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([29.0, 0.0], [-6.0, -10.0, -7.0], [12.0, 18.0, 10.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_chest",
                    PartPose::offset_and_rotation(-8.5, 3.0, 3.0, 0.0, 1.570796, 0.0),
                )
                .visible(false)
                .cube(cube([45.0, 28.0], [-3.0, 0.0, 0.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_chest",
                    PartPose::offset_and_rotation(5.5, 3.0, 3.0, 0.0, 1.570796, 0.0),
                )
                .visible(false)
                .cube(cube([45.0, 41.0], [-3.0, 0.0, 0.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.5, 10.0, 6.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.5, 10.0, 6.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.5, 10.0, -5.0))
                    .cube(cube([29.0, 29.0], [-2.0, 0.0, -2.0], [4.0, 14.0, 4.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(3.5, 10.0, -5.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_llama_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_llama_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_llama_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    let anim_value_0 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_0;
    let anim_value_1 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_1;
    let anim_value_2 = (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_2;
    let anim_value_3 =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_3;
    let anim_value_4 =
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_4;
    let anim_value_5 = (animation_pos * 0.6662).cos() * 1.4 * animation_speed;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_5;
    let anim_value_6 = state.has_chest;
    tree.part_mut_or_panic("right_chest").visible = anim_value_6;
    let anim_value_7 = state.has_chest;
    tree.part_mut_or_panic("left_chest").visible = anim_value_7;
}
pub fn apply_llama_llama_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_llama_model_setup_anim_ir(tree, state);
}
pub fn build_llama_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_llama_model_tree();
    apply_llama_llama_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_mangrove_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_mangrove_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_mangrove_boat_model_tree();
    apply_mangrove_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_mangrove_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_mangrove_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_mangrove_chest_boat_model_tree();
    apply_mangrove_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn apply_oak_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_oak_boat_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_oak_boat_model_tree();
    apply_oak_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_oak_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_oak_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_oak_chest_boat_model_tree();
    apply_oak_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_ocelot_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 15.0, -9.0))
                    .cube(cube([0.0, 0.0], [-2.5, -2.0, -3.0], [5.0, 4.0, 5.0]))
                    .cube(cube([0.0, 24.0], [-1.5, -0.001, -4.0], [3.0, 2.0, 2.0]))
                    .cube(cube([0.0, 10.0], [-2.0, -3.0, 0.0], [1.0, 1.0, 2.0]))
                    .cube(cube([6.0, 10.0], [1.0, -3.0, 0.0], [1.0, 1.0, 2.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 12.0, -10.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([20.0, 0.0], [-2.0, 3.0, -8.0], [4.0, 16.0, 6.0])),
            )
            .child(
                NamedPartSpec::new(
                    "tail1",
                    PartPose::offset_and_rotation(0.0, 15.0, 8.0, 0.9, 0.0, 0.0),
                )
                .cube(cube([0.0, 15.0], [-0.5, 0.0, 0.0], [1.0, 8.0, 1.0])),
            )
            .child(
                NamedPartSpec::new("tail2", PartPose::offset(0.0, 20.0, 14.0)).cube(cube_grown(
                    [4.0, 15.0],
                    [-0.5, 0.0, 0.0],
                    [1.0, 8.0, 1.0],
                    -0.02,
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(1.1, 18.0, 5.0)).cube(cube(
                    [8.0, 13.0],
                    [-1.0, 0.0, 1.0],
                    [2.0, 6.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-1.1, 18.0, 5.0)).cube(cube(
                    [8.0, 13.0],
                    [-1.0, 0.0, 1.0],
                    [2.0, 6.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(1.2, 14.1, -5.0)).cube(cube(
                    [40.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [2.0, 10.0, 2.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-1.2, 14.1, -5.0))
                    .cube(cube([40.0, 0.0], [-1.0, 0.0, 0.0], [2.0, 10.0, 2.0])),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_ocelot_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_ocelot_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_ocelot_adult_feline_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_adult_feline_model_setup_anim_ir(tree, state);
}
pub fn build_ocelot_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_ocelot_model_tree();
    apply_ocelot_adult_feline_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn apply_pale_oak_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_pale_oak_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_pale_oak_boat_model_tree();
    apply_pale_oak_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_pale_oak_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_pale_oak_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_pale_oak_chest_boat_model_tree();
    apply_pale_oak_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_panda_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 11.5, -17.0))
                    .cube(cube([0.0, 6.0], [-6.5, -5.0, -4.0], [13.0, 10.0, 9.0]))
                    .cube(cube([45.0, 16.0], [-3.5, 0.0, -6.0], [7.0, 5.0, 2.0]))
                    .cube(cube([52.0, 25.0], [3.5, -8.0, -1.0], [5.0, 4.0, 1.0]))
                    .cube(cube([52.0, 25.0], [-8.5, -8.0, -1.0], [5.0, 4.0, 1.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 10.0, 0.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([0.0, 25.0], [-9.5, -13.0, -6.5], [19.0, 26.0, 13.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-5.5, 15.0, 9.0)).cube(cube(
                    [40.0, 0.0],
                    [-3.0, 0.0, -3.0],
                    [6.0, 9.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(5.5, 15.0, 9.0)).cube(cube(
                    [40.0, 0.0],
                    [-3.0, 0.0, -3.0],
                    [6.0, 9.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-5.5, 15.0, -9.0))
                    .cube(cube([40.0, 0.0], [-3.0, 0.0, -3.0], [6.0, 9.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(5.5, 15.0, -9.0)).cube(cube(
                    [40.0, 0.0],
                    [-3.0, 0.0, -3.0],
                    [6.0, 9.0, 6.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_panda_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_panda_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_panda_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let internal_sneeze_pos = if (state.is_sneezing)
        && (!(state.sneeze_time < 15.0_f32) && (state.sneeze_time < 20.0_f32))
    {
        (state.sneeze_time - 15.0_f32) / 5.0_f32
    } else {
        0.0
    };
    let anim_value_0 = if state.is_unhappy {
        0.35 * (0.6 * state.age_in_ticks).sin()
    } else {
        tree.part_or_panic("head").pose.rotation.y
    };
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_0;
    let anim_value_1 = if state.is_unhappy {
        0.35 * (0.6 * state.age_in_ticks).sin()
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_1;
    let anim_value_2 = if state.is_unhappy {
        -0.75 * (0.3 * state.age_in_ticks).sin()
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_2;
    let anim_value_3 = if state.is_unhappy {
        0.75 * (0.3 * state.age_in_ticks).sin()
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_3;
    let anim_value_4 = if !(state.is_unhappy) {
        0.0
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_4;
    let anim_value_5 = if (state.is_sneezing) && (state.sneeze_time < 15.0_f32) {
        (-std::f32::consts::PI / 4.0_f32) * state.sneeze_time / 14.0
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_5;
    let anim_value_6 = if (state.is_sneezing)
        && (!(state.sneeze_time < 15.0_f32) && (state.sneeze_time < 20.0_f32))
    {
        (-std::f32::consts::PI / 4.0_f32)
            + (std::f32::consts::PI / 4.0_f32) * ((state.sneeze_time - 15.0_f32) / 5.0_f32)
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_6;
    let anim_value_7 = if state.sit_amount > 0.0 {
        tree.part_or_panic("body").pose.rotation.x
            + (state.sit_amount)
                * ((((1.7407963) - (tree.part_or_panic("body").pose.rotation.x))
                    + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI)
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_7;
    let anim_value_8 = if state.sit_amount > 0.0 {
        tree.part_or_panic("head").pose.rotation.x
            + (state.sit_amount)
                * ((((std::f32::consts::PI / 2.0_f32)
                    - (tree.part_or_panic("head").pose.rotation.x))
                    + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI)
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_8;
    let anim_value_9 = if state.sit_amount > 0.0 {
        -0.27079642
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.z = anim_value_9;
    let anim_value_10 = if state.sit_amount > 0.0 {
        0.27079642
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.z = anim_value_10;
    let anim_value_11 = if state.sit_amount > 0.0 {
        0.5707964
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.z = anim_value_11;
    let anim_value_12 = if state.sit_amount > 0.0 {
        -0.5707964
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.z = anim_value_12;
    let anim_value_13 = if (state.sit_amount > 0.0) && (state.is_eating) {
        (std::f32::consts::PI / 2.0_f32) + 0.2 * (state.age_in_ticks * 0.6).sin()
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_13;
    let anim_value_14 = if (state.sit_amount > 0.0) && (state.is_eating) {
        -0.4 - 0.2 * (state.age_in_ticks * 0.6).sin()
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_14;
    let anim_value_15 = if (state.sit_amount > 0.0) && (state.is_eating) {
        -0.4 - 0.2 * (state.age_in_ticks * 0.6).sin()
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_15;
    let anim_value_16 = if (state.sit_amount > 0.0) && (state.is_scared) {
        2.1707964
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_16;
    let anim_value_17 = if (state.sit_amount > 0.0) && (state.is_scared) {
        -0.9
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_17;
    let anim_value_18 = if (state.sit_amount > 0.0) && (state.is_scared) {
        -0.9
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_18;
    let anim_value_19 = if !(state.sit_amount > 0.0) {
        0.0
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.z = anim_value_19;
    let anim_value_20 = if !(state.sit_amount > 0.0) {
        0.0
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.z = anim_value_20;
    let anim_value_21 = if !(state.sit_amount > 0.0) {
        0.0
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.z = anim_value_21;
    let anim_value_22 = if !(state.sit_amount > 0.0) {
        0.0
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.z = anim_value_22;
    let anim_value_23 = if state.lie_on_back_amount > 0.0 {
        -0.6 * (state.age_in_ticks * 0.15).sin()
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_23;
    let anim_value_24 = if state.lie_on_back_amount > 0.0 {
        0.6 * (state.age_in_ticks * 0.15).sin()
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_24;
    let anim_value_25 = if state.lie_on_back_amount > 0.0 {
        0.3 * (state.age_in_ticks * 0.25).sin()
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_25;
    let anim_value_26 = if state.lie_on_back_amount > 0.0 {
        -0.3 * (state.age_in_ticks * 0.25).sin()
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_26;
    let anim_value_27 = if state.lie_on_back_amount > 0.0 {
        tree.part_or_panic("head").pose.rotation.x
            + (state.lie_on_back_amount)
                * ((((std::f32::consts::PI / 2.0_f32)
                    - (tree.part_or_panic("head").pose.rotation.x))
                    + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI)
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_27;
    let anim_value_28 = if state.roll_amount > 0.0 {
        tree.part_or_panic("head").pose.rotation.x
            + (state.roll_amount)
                * ((((2.0561945) - (tree.part_or_panic("head").pose.rotation.x))
                    + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI)
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_28;
    let anim_value_29 = if state.roll_amount > 0.0 {
        -0.5 * (state.age_in_ticks * 0.5).sin()
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_29;
    let anim_value_30 = if state.roll_amount > 0.0 {
        0.5 * (state.age_in_ticks * 0.5).sin()
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_30;
    let anim_value_31 = if state.roll_amount > 0.0 {
        0.5 * (state.age_in_ticks * 0.5).sin()
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_31;
    let anim_value_32 = if state.roll_amount > 0.0 {
        -0.5 * (state.age_in_ticks * 0.5).sin()
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_32;
}
pub fn apply_panda_panda_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
    apply_panda_model_setup_anim_ir(tree, state);
}
pub fn build_panda_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_panda_model_tree();
    apply_panda_panda_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_parrot_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 16.5, -3.0, 0.4937, 0.0, 0.0),
                )
                .cube(cube([2.0, 8.0], [-1.5, 0.0, -1.5], [3.0, 6.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(0.0, 21.07, 1.16, 1.015, 0.0, 0.0),
                )
                .cube(cube([22.0, 1.0], [-1.5, -1.0, -1.0], [3.0, 4.0, 1.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_wing",
                    PartPose::offset_and_rotation(
                        1.5,
                        16.94,
                        -2.76,
                        -0.6981,
                        -std::f32::consts::PI,
                        0.0,
                    ),
                )
                .cube(cube([19.0, 8.0], [-0.5, 0.0, -1.5], [1.0, 5.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_wing",
                    PartPose::offset_and_rotation(
                        -1.5,
                        16.94,
                        -2.76,
                        -0.6981,
                        -std::f32::consts::PI,
                        0.0,
                    ),
                )
                .cube(cube([19.0, 8.0], [-0.5, 0.0, -1.5], [1.0, 5.0, 3.0])),
            )
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 15.69, -2.76))
                    .cube(cube([2.0, 2.0], [-1.0, -1.5, -1.0], [2.0, 3.0, 2.0]))
                    .child(
                        NamedPartSpec::new("head2", PartPose::offset(0.0, -2.0, -1.0)).cube(cube(
                            [10.0, 0.0],
                            [-1.0, -0.5, -2.0],
                            [2.0, 1.0, 4.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("beak1", PartPose::offset(0.0, -0.5, -1.5)).cube(cube(
                            [11.0, 7.0],
                            [-0.5, -1.0, -0.5],
                            [1.0, 2.0, 1.0],
                        )),
                    )
                    .child(
                        NamedPartSpec::new("beak2", PartPose::offset(0.0, -1.75, -2.45))
                            .cube(cube([16.0, 7.0], [-0.5, 0.0, -0.5], [1.0, 2.0, 1.0])),
                    )
                    .child(
                        NamedPartSpec::new(
                            "feather",
                            PartPose::offset_and_rotation(0.0, -2.15, 0.15, -0.2214, 0.0, 0.0),
                        )
                        .cube(cube(
                            [2.0, 18.0],
                            [0.0, -4.0, -2.0],
                            [0.0, 5.0, 4.0],
                        )),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "left_leg",
                    PartPose::offset_and_rotation(1.0, 22.0, -1.05, -0.0299, 0.0, 0.0),
                )
                .cube(cube([14.0, 18.0], [-0.5, 0.0, -0.5], [1.0, 2.0, 1.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_leg",
                    PartPose::offset_and_rotation(-1.0, 22.0, -1.05, -0.0299, 0.0, 0.0),
                )
                .cube(cube([14.0, 18.0], [-0.5, 0.0, -0.5], [1.0, 2.0, 1.0])),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_parrot_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_parrot_model_tree().mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn apply_parrot_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_pos = state.walk_animation_pos;
    let anim_speed = state.walk_animation_speed;
    let flap_angle = (state.flap.sin() + 1.0) * state.flap_speed;
    let bobbing_body = flap_angle * 0.3;
    if let Some(h) = tree.part_mut("head") {
        h.pose.rotation.x = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.rotation.y = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
        h.pose.offset.y += bobbing_body;
    }
    if let Some(t) = tree.part_mut("tail") {
        t.pose.rotation.x += (anim_pos * 0.6662).cos() * 0.3 * anim_speed;
        t.pose.offset.y += bobbing_body;
    }
    if let Some(b) = tree.part_mut("body") {
        b.pose.offset.y += bobbing_body;
    }
    if let Some(lw) = tree.part_mut("left_wing") {
        lw.pose.rotation.z = -0.0873 - flap_angle;
        lw.pose.offset.y += bobbing_body;
    }
    if let Some(rw) = tree.part_mut("right_wing") {
        rw.pose.rotation.z = 0.0873 + flap_angle;
        rw.pose.offset.y += bobbing_body;
    }
    if let Some(ll) = tree.part_mut("left_leg") {
        ll.pose.rotation.x += (anim_pos * 0.6662).cos() * 1.4 * anim_speed;
        ll.pose.offset.y += bobbing_body;
    }
    if let Some(rl) = tree.part_mut("right_leg") {
        rl.pose.rotation.x += (anim_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * anim_speed;
        rl.pose.offset.y += bobbing_body;
    }
}
pub fn build_parrot_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_parrot_model_tree();
    tree.reset_pose();
    apply_parrot_model_setup_anim_ir(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 32.0,
        height: 32.0,
    })
}
pub fn build_phantom_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO).child(
        NamedPartSpec::new(
            "body",
            PartPose::offset_and_rotation(0.0, 0.0, 0.0, -0.1, 0.0, 0.0),
        )
        .cube(cube([0.0, 8.0], [-3.0, -2.0, -8.0], [5.0, 3.0, 9.0]))
        .child(
            NamedPartSpec::new("tail_base", PartPose::offset(0.0, -2.0, 1.0))
                .cube(cube([3.0, 20.0], [-2.0, 0.0, 0.0], [3.0, 2.0, 6.0]))
                .child(
                    NamedPartSpec::new("tail_tip", PartPose::offset(0.0, 0.5, 6.0)).cube(cube(
                        [4.0, 29.0],
                        [-1.0, 0.0, 0.0],
                        [1.0, 1.0, 6.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "left_wing_base",
                PartPose::offset_and_rotation(2.0, -2.0, -8.0, 0.0, 0.0, 0.1),
            )
            .cube(cube([23.0, 12.0], [0.0, 0.0, 0.0], [6.0, 2.0, 9.0]))
            .child(
                NamedPartSpec::new(
                    "left_wing_tip",
                    PartPose::offset_and_rotation(6.0, 0.0, 0.0, 0.0, 0.0, 0.1),
                )
                .cube(cube([16.0, 24.0], [0.0, 0.0, 0.0], [13.0, 1.0, 9.0])),
            ),
        )
        .child(
            NamedPartSpec::new(
                "right_wing_base",
                PartPose::offset_and_rotation(-3.0, -2.0, -8.0, 0.0, 0.0, -0.1),
            )
            .cube(cube_mirrored(
                [23.0, 12.0],
                [-6.0, 0.0, 0.0],
                [6.0, 2.0, 9.0],
            ))
            .child(
                NamedPartSpec::new(
                    "right_wing_tip",
                    PartPose::offset_and_rotation(-6.0, 0.0, 0.0, 0.0, 0.0, -0.1),
                )
                .cube(cube_mirrored(
                    [16.0, 24.0],
                    [-13.0, 0.0, 0.0],
                    [13.0, 1.0, 9.0],
                )),
            ),
        )
        .child(
            NamedPartSpec::new(
                "head",
                PartPose::offset_and_rotation(0.0, 1.0, -7.0, 0.2, 0.0, 0.0),
            )
            .cube(cube([0.0, 0.0], [-4.0, -2.0, -5.0], [7.0, 3.0, 5.0])),
        ),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_phantom_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_phantom_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_phantom_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim = state.flap_time * 7.448451 * (std::f32::consts::PI / 180.0);
    let flap_amount = 16.0;
    let anim_value_0 = (anim).cos() * 16.0 * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("left_wing_base").pose.rotation.z = anim_value_0;
    let anim_value_1 = (anim).cos() * 16.0 * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("left_wing_tip").pose.rotation.z = anim_value_1;
    let anim_value_2 = -tree.part_or_panic("left_wing_base").pose.rotation.z;
    tree.part_mut_or_panic("right_wing_base").pose.rotation.z = anim_value_2;
    let anim_value_3 = -tree.part_or_panic("left_wing_tip").pose.rotation.z;
    tree.part_mut_or_panic("right_wing_tip").pose.rotation.z = anim_value_3;
    let anim_value_4 = -(5.0 + (anim * 2.0).cos() * 5.0) * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("tail_base").pose.rotation.x = anim_value_4;
    let anim_value_5 = -(5.0 + (anim * 2.0).cos() * 5.0) * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("tail_tip").pose.rotation.x = anim_value_5;
}
pub fn apply_phantom_phantom_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_phantom_model_setup_anim_ir(tree, state);
}
pub fn build_phantom_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_phantom_model_tree();
    apply_phantom_phantom_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_polar_bear_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new(
        "root",
        PartPose {
            offset: Vec3::new(0.0, -4.8032, 0.0),
            rotation: Vec3::splat(0.0),
            scale: Vec3::splat(1.2),
        },
    )
    .child(
        NamedPartSpec::new("head", PartPose::offset(0.0, 10.0, -16.0))
            .cube(cube([0.0, 0.0], [-3.5, -3.0, -3.0], [7.0, 7.0, 7.0]))
            .cube(cube([0.0, 44.0], [-2.5, 1.0, -6.0], [5.0, 3.0, 3.0]))
            .cube(cube([26.0, 0.0], [-4.5, -4.0, -1.0], [2.0, 2.0, 1.0]))
            .cube(cube_mirrored(
                [26.0, 0.0],
                [2.5, -4.0, -1.0],
                [2.0, 2.0, 1.0],
            )),
    )
    .child(
        NamedPartSpec::new(
            "body",
            PartPose::offset_and_rotation(-2.0, 9.0, 12.0, 1.570796, 0.0, 0.0),
        )
        .cube(cube([0.0, 19.0], [-5.0, -13.0, -7.0], [14.0, 14.0, 11.0]))
        .cube(cube([39.0, 0.0], [-4.0, -25.0, -7.0], [12.0, 12.0, 10.0])),
    )
    .child(
        NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.5, 14.0, 6.0)).cube(cube(
            [50.0, 22.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 8.0],
        )),
    )
    .child(
        NamedPartSpec::new("left_hind_leg", PartPose::offset(4.5, 14.0, 6.0)).cube(cube(
            [50.0, 22.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 8.0],
        )),
    )
    .child(
        NamedPartSpec::new("right_front_leg", PartPose::offset(-3.5, 14.0, -8.0)).cube(cube(
            [50.0, 40.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 6.0],
        )),
    )
    .child(
        NamedPartSpec::new("left_front_leg", PartPose::offset(3.5, 14.0, -8.0)).cube(cube(
            [50.0, 40.0],
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, 6.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_polar_bear_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_polar_bear_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_polar_bear_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let stand_scale = state.stand_scale * state.stand_scale;
    let body_age_scale = state.age_scale;
    let anim_value_0 = stand_scale * std::f32::consts::PI * 0.35;
    tree.part_mut_or_panic("body").pose.rotation.x -= anim_value_0;
    let anim_value_1 = stand_scale * body_age_scale * 2.0;
    tree.part_mut_or_panic("body").pose.offset.y += anim_value_1;
    let anim_value_2 = stand_scale * body_age_scale * 20.0;
    tree.part_mut_or_panic("right_front_leg").pose.offset.y -= anim_value_2;
    let anim_value_3 = stand_scale * body_age_scale * 4.0;
    tree.part_mut_or_panic("right_front_leg").pose.offset.z += anim_value_3;
    let anim_value_4 = stand_scale * std::f32::consts::PI * 0.45;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x -= anim_value_4;
    let anim_value_5 = tree.part_or_panic("right_front_leg").pose.offset.y;
    tree.part_mut_or_panic("left_front_leg").pose.offset.y = anim_value_5;
    let anim_value_6 = tree.part_or_panic("right_front_leg").pose.offset.z;
    tree.part_mut_or_panic("left_front_leg").pose.offset.z = anim_value_6;
    let anim_value_7 = stand_scale * std::f32::consts::PI * 0.45;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x -= anim_value_7;
    let anim_value_8 = stand_scale * 24.0;
    tree.part_mut_or_panic("head").pose.offset.y -= anim_value_8;
    let anim_value_9 = stand_scale * 13.0;
    tree.part_mut_or_panic("head").pose.offset.z += anim_value_9;
    let anim_value_10 = stand_scale * std::f32::consts::PI * 0.15;
    tree.part_mut_or_panic("head").pose.rotation.x += anim_value_10;
}
pub fn apply_polar_bear_polar_bear_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
    apply_polar_bear_model_setup_anim_ir(tree, state);
}
pub fn build_polar_bear_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_polar_bear_model_tree();
    apply_polar_bear_polar_bear_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_sheep_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::offset(0.0, 6.0, -8.0)).cube(cube(
                [0.0, 0.0],
                [-3.0, -4.0, -6.0],
                [6.0, 6.0, 8.0],
            )),
        )
        .child(
            NamedPartSpec::new(
                "body",
                PartPose::offset_and_rotation(0.0, 5.0, 2.0, 1.570796, 0.0, 0.0),
            )
            .cube(cube([28.0, 8.0], [-4.0, -10.0, -7.0], [8.0, 16.0, 6.0])),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.0, 12.0, 7.0)).cube(
                cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(3.0, 12.0, 7.0)).cube(cube(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-3.0, 12.0, -5.0)).cube(
                cube_mirrored([0.0, 16.0], [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(3.0, 12.0, -5.0)).cube(cube(
                [0.0, 16.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn apply_sheep_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = tree.part_or_panic("head").pose.offset.y
        + state.head_eat_position_scale * 9.0 * state.age_scale;
    tree.part_mut_or_panic("head").pose.offset.y = anim_value_0;
    let anim_value_1 = state.head_eat_angle_scale;
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_1;
}
pub fn apply_sheep_sheep_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
    apply_sheep_model_setup_anim_ir(tree, state);
}
pub fn build_shulker_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("lid", PartPose::offset(0.0, 24.0, 0.0)).cube(cube(
                [0.0, 0.0],
                [-8.0, -16.0, -8.0],
                [16.0, 12.0, 16.0],
            )),
        )
        .child(
            NamedPartSpec::new("base", PartPose::offset(0.0, 24.0, 0.0)).cube(cube(
                [0.0, 28.0],
                [-8.0, -8.0, -8.0],
                [16.0, 8.0, 16.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_shulker_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_shulker_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_shulker_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let bs = (0.5 + state.peek_amount) * std::f32::consts::PI;
    let q = -1.0 + (bs).sin();
    let mut extra = 0.0;
    if bs > std::f32::consts::PI {
        extra = (state.age_in_ticks * 0.1).sin() * 0.7;
    }
    let anim_value_0 = 0.0;
    tree.part_mut_or_panic("lid").pose.offset.x = anim_value_0;
    let anim_value_1 = 16.0 + (bs).sin() * 8.0 + extra;
    tree.part_mut_or_panic("lid").pose.offset.y = anim_value_1;
    let anim_value_2 = 0.0;
    tree.part_mut_or_panic("lid").pose.offset.z = anim_value_2;
    let anim_value_3 = if state.peek_amount > 0.3 {
        q * q * q * q * std::f32::consts::PI * 0.125
    } else {
        tree.part_or_panic("lid").pose.rotation.y
    };
    tree.part_mut_or_panic("lid").pose.rotation.y = anim_value_3;
    let anim_value_4 = if !(state.peek_amount > 0.3) {
        0.0
    } else {
        tree.part_or_panic("lid").pose.rotation.y
    };
    tree.part_mut_or_panic("lid").pose.rotation.y = anim_value_4;
    let anim_value_5 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_5;
    let anim_value_6 =
        (state.y_head_rot - 180.0 - state.y_body_rot) * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_6;
}
pub fn apply_shulker_shulker_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_shulker_model_setup_anim_ir(tree, state);
}
pub fn build_shulker_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_shulker_model_tree();
    apply_shulker_shulker_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_spruce_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_spruce_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_spruce_boat_model_tree();
    apply_spruce_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_spruce_chest_boat_boat_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_boat_model_setup_anim_ir(tree, state);
}
pub fn build_spruce_chest_boat_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_spruce_chest_boat_model_tree();
    apply_spruce_chest_boat_boat_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_trader_llama_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 7.0, -6.0))
                    .cube(cube([0.0, 0.0], [-2.0, -14.0, -10.0], [4.0, 4.0, 9.0]))
                    .cube(cube([0.0, 14.0], [-4.0, -16.0, -6.0], [8.0, 18.0, 6.0]))
                    .cube(cube([17.0, 0.0], [-4.0, -19.0, -4.0], [3.0, 3.0, 2.0]))
                    .cube(cube([17.0, 0.0], [1.0, -19.0, -4.0], [3.0, 3.0, 2.0])),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 5.0, 2.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([29.0, 0.0], [-6.0, -10.0, -7.0], [12.0, 18.0, 10.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_chest",
                    PartPose::offset_and_rotation(-8.5, 3.0, 3.0, 0.0, 1.570796, 0.0),
                )
                .visible(false)
                .cube(cube([45.0, 28.0], [-3.0, 0.0, 0.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_chest",
                    PartPose::offset_and_rotation(5.5, 3.0, 3.0, 0.0, 1.570796, 0.0),
                )
                .visible(false)
                .cube(cube([45.0, 41.0], [-3.0, 0.0, 0.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.5, 10.0, 6.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.5, 10.0, 6.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-3.5, 10.0, -5.0))
                    .cube(cube([29.0, 29.0], [-2.0, 0.0, -2.0], [4.0, 14.0, 4.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(3.5, 10.0, -5.0)).cube(cube(
                    [29.0, 29.0],
                    [-2.0, 0.0, -2.0],
                    [4.0, 14.0, 4.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_trader_llama_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_trader_llama_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_trader_llama_llama_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_llama_model_setup_anim_ir(tree, state);
}
pub fn build_trader_llama_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_trader_llama_model_tree();
    apply_trader_llama_llama_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_villager_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 0.0], [-4.0, -10.0, -4.0], [8.0, 10.0, 8.0]))
                .child(
                    NamedPartSpec::new("hat", PartPose::ZERO)
                        .cube(cube_grown(
                            [32.0, 0.0],
                            [-4.0, -10.0, -4.0],
                            [8.0, 10.0, 8.0],
                            0.51,
                        ))
                        .child(
                            NamedPartSpec::new(
                                "hat_rim",
                                PartPose::offset_and_rotation(0.0, 0.0, 0.0, -1.570796, 0.0, 0.0),
                            )
                            .cube(cube(
                                [30.0, 47.0],
                                [-8.0, -8.0, -6.0],
                                [16.0, 16.0, 1.0],
                            )),
                        ),
                )
                .child(
                    NamedPartSpec::new("nose", PartPose::offset(0.0, -2.0, 0.0)).cube(cube(
                        [24.0, 0.0],
                        [-1.0, -1.0, -6.0],
                        [2.0, 4.0, 2.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new("body", PartPose::ZERO)
                .cube(cube([16.0, 20.0], [-4.0, 0.0, -3.0], [8.0, 12.0, 6.0]))
                .child(
                    NamedPartSpec::new("jacket", PartPose::ZERO).cube(cube_grown(
                        [0.0, 38.0],
                        [-4.0, 0.0, -3.0],
                        [8.0, 20.0, 6.0],
                        0.5,
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "arms",
                PartPose::offset_and_rotation(0.0, 3.0, -1.0, -0.75, 0.0, 0.0),
            )
            .cube(cube([44.0, 22.0], [-8.0, -2.0, -2.0], [4.0, 8.0, 4.0]))
            .cube(cube_mirrored(
                [44.0, 22.0],
                [4.0, -2.0, -2.0],
                [4.0, 8.0, 4.0],
            ))
            .cube(cube([40.0, 38.0], [-4.0, 2.0, -2.0], [8.0, 4.0, 4.0])),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_villager_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_villager_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_villager_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_1;
    let anim_value_2 = if state.is_unhappy {
        0.3 * (0.45 * state.age_in_ticks).sin()
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_2;
    let anim_value_3 = if state.is_unhappy {
        0.4
    } else {
        tree.part_or_panic("head").pose.rotation.x
    };
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_3;
    let anim_value_4 = if !(state.is_unhappy) {
        0.0
    } else {
        tree.part_or_panic("head").pose.rotation.z
    };
    tree.part_mut_or_panic("head").pose.rotation.z = anim_value_4;
    let anim_value_5 =
        (state.walk_animation_pos * 0.6662).cos() * 1.4 * state.walk_animation_speed * 0.5;
    tree.part_mut_or_panic("right_leg").pose.rotation.x = anim_value_5;
    let anim_value_6 = (state.walk_animation_pos * 0.6662 + std::f32::consts::PI).cos()
        * 1.4
        * state.walk_animation_speed
        * 0.5;
    tree.part_mut_or_panic("left_leg").pose.rotation.x = anim_value_6;
    let anim_value_7 = 0.0;
    tree.part_mut_or_panic("right_leg").pose.rotation.y = anim_value_7;
    let anim_value_8 = 0.0;
    tree.part_mut_or_panic("left_leg").pose.rotation.y = anim_value_8;
}
pub fn apply_villager_villager_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_villager_model_setup_anim_ir(tree, state);
}
pub fn build_villager_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_villager_model_tree();
    apply_villager_villager_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_wandering_trader_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 0.0], [-4.0, -10.0, -4.0], [8.0, 10.0, 8.0]))
                .child(
                    NamedPartSpec::new("hat", PartPose::ZERO)
                        .cube(cube_grown(
                            [32.0, 0.0],
                            [-4.0, -10.0, -4.0],
                            [8.0, 10.0, 8.0],
                            0.51,
                        ))
                        .child(
                            NamedPartSpec::new(
                                "hat_rim",
                                PartPose::offset_and_rotation(0.0, 0.0, 0.0, -1.570796, 0.0, 0.0),
                            )
                            .cube(cube(
                                [30.0, 47.0],
                                [-8.0, -8.0, -6.0],
                                [16.0, 16.0, 1.0],
                            )),
                        ),
                )
                .child(
                    NamedPartSpec::new("nose", PartPose::offset(0.0, -2.0, 0.0)).cube(cube(
                        [24.0, 0.0],
                        [-1.0, -1.0, -6.0],
                        [2.0, 4.0, 2.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new("body", PartPose::ZERO)
                .cube(cube([16.0, 20.0], [-4.0, 0.0, -3.0], [8.0, 12.0, 6.0]))
                .child(
                    NamedPartSpec::new("jacket", PartPose::ZERO).cube(cube_grown(
                        [0.0, 38.0],
                        [-4.0, 0.0, -3.0],
                        [8.0, 20.0, 6.0],
                        0.5,
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "arms",
                PartPose::offset_and_rotation(0.0, 3.0, -1.0, -0.75, 0.0, 0.0),
            )
            .cube(cube([44.0, 22.0], [-8.0, -2.0, -2.0], [4.0, 8.0, 4.0]))
            .cube(cube_mirrored(
                [44.0, 22.0],
                [4.0, -2.0, -2.0],
                [4.0, 8.0, 4.0],
            ))
            .cube(cube([40.0, 38.0], [-4.0, 2.0, -2.0], [8.0, 4.0, 4.0])),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_wandering_trader_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_wandering_trader_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_wandering_trader_villager_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_villager_model_setup_anim_ir(tree, state);
}
pub fn build_wandering_trader_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wandering_trader_model_tree();
    apply_wandering_trader_villager_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_witch_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 0.0], [-4.0, -10.0, -4.0], [8.0, 10.0, 8.0]))
                .child(
                    NamedPartSpec::new("hat", PartPose::offset(-5.0, -10.03125, -5.0))
                        .cube(cube([0.0, 64.0], [0.0, 0.0, 0.0], [10.0, 2.0, 10.0]))
                        .child(
                            NamedPartSpec::new(
                                "hat_rim",
                                PartPose::offset_and_rotation(0.0, 0.0, 0.0, -1.570796, 0.0, 0.0),
                            )
                            .cube(cube(
                                [30.0, 47.0],
                                [-8.0, -8.0, -6.0],
                                [16.0, 16.0, 1.0],
                            )),
                        )
                        .child(
                            NamedPartSpec::new(
                                "hat2",
                                PartPose::offset_and_rotation(
                                    1.75, -4.0, 2.0, -0.05236, 0.0, 0.02618,
                                ),
                            )
                            .cube(cube([0.0, 76.0], [0.0, 0.0, 0.0], [7.0, 4.0, 7.0]))
                            .child(
                                NamedPartSpec::new(
                                    "hat3",
                                    PartPose::offset_and_rotation(
                                        1.75, -4.0, 2.0, -0.10472, 0.0, 0.05236,
                                    ),
                                )
                                .cube(cube([0.0, 87.0], [0.0, 0.0, 0.0], [4.0, 4.0, 4.0]))
                                .child(
                                    NamedPartSpec::new(
                                        "hat4",
                                        PartPose::offset_and_rotation(
                                            1.75, -2.0, 2.0, -0.20944, 0.0, 0.10472,
                                        ),
                                    )
                                    .cube(cube_grown(
                                        [0.0, 95.0],
                                        [0.0, 0.0, 0.0],
                                        [1.0, 2.0, 1.0],
                                        0.25,
                                    )),
                                ),
                            ),
                        ),
                )
                .child(
                    NamedPartSpec::new("nose", PartPose::offset(0.0, -2.0, 0.0))
                        .cube(cube([24.0, 0.0], [-1.0, -1.0, -6.0], [2.0, 4.0, 2.0]))
                        .child(
                            NamedPartSpec::new("mole", PartPose::offset(0.0, -2.0, 0.0)).cube(
                                cube_grown([0.0, 0.0], [0.0, 3.0, -6.75], [1.0, 1.0, 1.0], -0.25),
                            ),
                        ),
                ),
        )
        .child(
            NamedPartSpec::new("body", PartPose::ZERO)
                .cube(cube([16.0, 20.0], [-4.0, 0.0, -3.0], [8.0, 12.0, 6.0]))
                .child(
                    NamedPartSpec::new("jacket", PartPose::ZERO).cube(cube_grown(
                        [0.0, 38.0],
                        [-4.0, 0.0, -3.0],
                        [8.0, 20.0, 6.0],
                        0.5,
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "arms",
                PartPose::offset_and_rotation(0.0, 3.0, -1.0, -0.75, 0.0, 0.0),
            )
            .cube(cube([44.0, 22.0], [-8.0, -2.0, -2.0], [4.0, 8.0, 4.0]))
            .cube(cube_mirrored(
                [44.0, 22.0],
                [4.0, -2.0, -2.0],
                [4.0, 8.0, 4.0],
            ))
            .cube(cube([40.0, 38.0], [-4.0, 2.0, -2.0], [8.0, 4.0, 4.0])),
        )
        .child(
            NamedPartSpec::new("right_leg", PartPose::offset(-2.0, 12.0, 0.0)).cube(cube(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_leg", PartPose::offset(2.0, 12.0, 0.0)).cube(cube_mirrored(
                [0.0, 22.0],
                [-2.0, 0.0, -2.0],
                [4.0, 12.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_witch_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_witch_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 128.0,
    })
}
pub fn apply_witch_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let speed = 0.01 * (state.entity_id % 10.0_f32);
    let anim_value_0 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_0;
    let anim_value_1 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_1;
    let anim_value_2 =
        (state.walk_animation_pos * 0.6662).cos() * 1.4 * state.walk_animation_speed * 0.5;
    tree.part_mut_or_panic("right_leg").pose.rotation.x = anim_value_2;
    let anim_value_3 = (state.walk_animation_pos * 0.6662 + std::f32::consts::PI).cos()
        * 1.4
        * state.walk_animation_speed
        * 0.5;
    tree.part_mut_or_panic("left_leg").pose.rotation.x = anim_value_3;
    let anim_value_4 = (state.age_in_ticks * speed).sin() * 4.5 * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("nose").pose.rotation.x = anim_value_4;
    let anim_value_5 = (state.age_in_ticks * speed).cos() * 2.5 * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("nose").pose.rotation.z = anim_value_5;
    let anim_value_6 = if state.is_holding_item {
        0.0
    } else {
        tree.part_or_panic("nose").pose.offset.x
    };
    tree.part_mut_or_panic("nose").pose.offset.x = anim_value_6;
    let anim_value_7 = if state.is_holding_item {
        1.0
    } else {
        tree.part_or_panic("nose").pose.offset.y
    };
    tree.part_mut_or_panic("nose").pose.offset.y = anim_value_7;
    let anim_value_8 = if state.is_holding_item {
        -1.5
    } else {
        tree.part_or_panic("nose").pose.offset.z
    };
    tree.part_mut_or_panic("nose").pose.offset.z = anim_value_8;
    let anim_value_9 = if state.is_holding_item {
        -0.9
    } else {
        tree.part_or_panic("nose").pose.rotation.x
    };
    tree.part_mut_or_panic("nose").pose.rotation.x = anim_value_9;
}
pub fn apply_witch_witch_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_witch_model_setup_anim_ir(tree, state);
}
pub fn build_witch_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_witch_model_tree();
    apply_witch_witch_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 128.0,
    })
}
pub fn build_zoglin_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("body", PartPose::offset(0.0, 7.0, 0.0))
                    .cube(cube([1.0, 1.0], [-8.0, -7.0, -13.0], [16.0, 14.0, 26.0]))
                    .child(
                        NamedPartSpec::new("mane", PartPose::offset(0.0, -14.0, -7.0)).cube(
                            cube_grown([90.0, 33.0], [0.0, 0.0, -9.0], [0.0, 10.0, 19.0], 0.001),
                        ),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "head",
                    PartPose::offset_and_rotation(0.0, 2.0, -12.0, 0.872665, 0.0, 0.0),
                )
                .cube(cube([61.0, 1.0], [-7.0, -3.0, -19.0], [14.0, 6.0, 19.0]))
                .child(
                    NamedPartSpec::new(
                        "right_ear",
                        PartPose::offset_and_rotation(-6.0, -2.0, -3.0, 0.0, 0.0, -0.698132),
                    )
                    .cube(cube(
                        [1.0, 1.0],
                        [-6.0, -1.0, -2.0],
                        [6.0, 1.0, 4.0],
                    )),
                )
                .child(
                    NamedPartSpec::new(
                        "left_ear",
                        PartPose::offset_and_rotation(6.0, -2.0, -3.0, 0.0, 0.0, 0.698132),
                    )
                    .cube(cube([1.0, 6.0], [0.0, -1.0, -2.0], [6.0, 1.0, 4.0])),
                )
                .child(
                    NamedPartSpec::new("right_horn", PartPose::offset(-7.0, 2.0, -12.0))
                        .cube(cube([10.0, 13.0], [-1.0, -11.0, -1.0], [2.0, 11.0, 2.0])),
                )
                .child(
                    NamedPartSpec::new("left_horn", PartPose::offset(7.0, 2.0, -12.0)).cube(cube(
                        [1.0, 13.0],
                        [-1.0, -11.0, -1.0],
                        [2.0, 11.0, 2.0],
                    )),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 10.0, -8.5))
                    .cube(cube([66.0, 42.0], [-3.0, 0.0, -3.0], [6.0, 14.0, 6.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 10.0, -8.5)).cube(cube(
                    [41.0, 42.0],
                    [-3.0, 0.0, -3.0],
                    [6.0, 14.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-5.0, 13.0, 10.0))
                    .cube(cube([21.0, 45.0], [-2.5, 0.0, -2.5], [5.0, 11.0, 5.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(5.0, 13.0, 10.0)).cube(cube(
                    [0.0, 45.0],
                    [-2.5, 0.0, -2.5],
                    [5.0, 11.0, 5.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_zoglin_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_zoglin_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_zoglin_hoglin_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_hoglin_model_setup_anim_ir(tree, state);
}
pub fn build_zoglin_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_zoglin_model_tree();
    apply_zoglin_hoglin_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_wolf_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_pos = state.walk_animation_pos;
    let animation_speed = state.walk_animation_speed;
    let anim_value_0 = if state.is_angry {
        0.0
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_0;
    let anim_value_1 = if !(state.is_angry) {
        (animation_pos * 0.6662).cos() * 1.4 * animation_speed
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_1;
    let anim_value_2 = if state.is_sitting {
        tree.part_or_panic("body").pose.offset.y + (4.0 * state.age_scale)
    } else {
        tree.part_or_panic("body").pose.offset.y
    };
    tree.part_mut_or_panic("body").pose.offset.y = anim_value_2;
    let anim_value_3 = if state.is_sitting {
        tree.part_or_panic("body").pose.offset.z - (2.0 * state.age_scale)
    } else {
        tree.part_or_panic("body").pose.offset.z
    };
    tree.part_mut_or_panic("body").pose.offset.z = anim_value_3;
    let anim_value_4 = if state.is_sitting {
        std::f32::consts::PI / 4.0_f32
    } else {
        tree.part_or_panic("body").pose.rotation.x
    };
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_4;
    let anim_value_5 = if state.is_sitting {
        tree.part_or_panic("tail").pose.offset.y + (9.0 * state.age_scale)
    } else {
        tree.part_or_panic("tail").pose.offset.y
    };
    tree.part_mut_or_panic("tail").pose.offset.y = anim_value_5;
    let anim_value_6 = if state.is_sitting {
        tree.part_or_panic("tail").pose.offset.z - (2.0 * state.age_scale)
    } else {
        tree.part_or_panic("tail").pose.offset.z
    };
    tree.part_mut_or_panic("tail").pose.offset.z = anim_value_6;
    let anim_value_7 = if state.is_sitting {
        tree.part_or_panic("right_hind_leg").pose.offset.y + (6.7 * state.age_scale)
    } else {
        tree.part_or_panic("right_hind_leg").pose.offset.y
    };
    tree.part_mut_or_panic("right_hind_leg").pose.offset.y = anim_value_7;
    let anim_value_8 = if state.is_sitting {
        tree.part_or_panic("right_hind_leg").pose.offset.z - (5.0 * state.age_scale)
    } else {
        tree.part_or_panic("right_hind_leg").pose.offset.z
    };
    tree.part_mut_or_panic("right_hind_leg").pose.offset.z = anim_value_8;
    let anim_value_9 = if state.is_sitting {
        std::f32::consts::PI * 3.0 / 2.0
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_9;
    let anim_value_10 = if state.is_sitting {
        tree.part_or_panic("left_hind_leg").pose.offset.y + (6.7 * state.age_scale)
    } else {
        tree.part_or_panic("left_hind_leg").pose.offset.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.offset.y = anim_value_10;
    let anim_value_11 = if state.is_sitting {
        tree.part_or_panic("left_hind_leg").pose.offset.z - (5.0 * state.age_scale)
    } else {
        tree.part_or_panic("left_hind_leg").pose.offset.z
    };
    tree.part_mut_or_panic("left_hind_leg").pose.offset.z = anim_value_11;
    let anim_value_12 = if state.is_sitting {
        std::f32::consts::PI * 3.0 / 2.0
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_12;
    let anim_value_13 = if state.is_sitting {
        5.811947
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_13;
    let anim_value_14 = if state.is_sitting {
        tree.part_or_panic("right_front_leg").pose.offset.x + (0.01 * state.age_scale)
    } else {
        tree.part_or_panic("right_front_leg").pose.offset.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.offset.x = anim_value_14;
    let anim_value_15 = if state.is_sitting {
        tree.part_or_panic("right_front_leg").pose.offset.y + (1.0 * state.age_scale)
    } else {
        tree.part_or_panic("right_front_leg").pose.offset.y
    };
    tree.part_mut_or_panic("right_front_leg").pose.offset.y = anim_value_15;
    let anim_value_16 = if state.is_sitting {
        5.811947
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_16;
    let anim_value_17 = if state.is_sitting {
        tree.part_or_panic("left_front_leg").pose.offset.x - (0.01 * state.age_scale)
    } else {
        tree.part_or_panic("left_front_leg").pose.offset.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.offset.x = anim_value_17;
    let anim_value_18 = if state.is_sitting {
        tree.part_or_panic("left_front_leg").pose.offset.y + (1.0 * state.age_scale)
    } else {
        tree.part_or_panic("left_front_leg").pose.offset.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.offset.y = anim_value_18;
    let anim_value_19 = if !(state.is_sitting) {
        (animation_pos * 0.6662).cos() * 1.4 * animation_speed
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_19;
    let anim_value_20 = if !(state.is_sitting) {
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_20;
    let anim_value_21 = if !(state.is_sitting) {
        (animation_pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * animation_speed
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_21;
    let anim_value_22 = if !(state.is_sitting) {
        (animation_pos * 0.6662).cos() * 1.4 * animation_speed
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_22;
    let anim_value_23 = ((((state.shake_anim + (-0.16)) / 1.8).clamp(0.0, 1.0))
        * std::f32::consts::PI)
        .sin()
        * ((((state.shake_anim + (-0.16)) / 1.8).clamp(0.0, 1.0)) * std::f32::consts::PI * 11.0)
            .sin()
        * 0.15
        * std::f32::consts::PI;
    tree.part_mut_or_panic("body").pose.rotation.z = anim_value_23;
    let anim_value_24 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_24;
    let anim_value_25 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_25;
    let anim_value_26 = state.tail_angle;
    tree.part_mut_or_panic("tail").pose.rotation.x = anim_value_26;
}
pub fn apply_wolf_adult_wolf_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_wolf_model_setup_anim_ir(tree, state);
}
pub fn build_wolf_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_wolf_model_tree();
    apply_wolf_adult_wolf_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 32.0,
    })
}
pub fn build_ravager_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("neck", PartPose::offset(0.0, -7.0, 5.5))
                    .cube(cube([68.0, 73.0], [-5.0, -1.0, -18.0], [10.0, 10.0, 18.0]))
                    .child(
                        NamedPartSpec::new("head", PartPose::offset(0.0, 16.0, -17.0))
                            .cube(cube([0.0, 0.0], [-8.0, -20.0, -14.0], [16.0, 20.0, 16.0]))
                            .cube(cube([0.0, 0.0], [-2.0, -6.0, -18.0], [4.0, 8.0, 4.0]))
                            .child(
                                NamedPartSpec::new(
                                    "right_horn",
                                    PartPose::offset_and_rotation(
                                        -10.0, -14.0, -8.0, 1.099557, 0.0, 0.0,
                                    ),
                                )
                                .cube(cube(
                                    [74.0, 55.0],
                                    [0.0, -14.0, -2.0],
                                    [2.0, 14.0, 4.0],
                                )),
                            )
                            .child(
                                NamedPartSpec::new(
                                    "left_horn",
                                    PartPose::offset_and_rotation(
                                        8.0, -14.0, -8.0, 1.099557, 0.0, 0.0,
                                    ),
                                )
                                .cube(cube_mirrored(
                                    [74.0, 55.0],
                                    [0.0, -14.0, -2.0],
                                    [2.0, 14.0, 4.0],
                                )),
                            )
                            .child(
                                NamedPartSpec::new("mouth", PartPose::offset(0.0, -2.0, 2.0))
                                    .cube(cube([0.0, 36.0], [-8.0, 0.0, -16.0], [16.0, 3.0, 16.0])),
                            ),
                    ),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 1.0, 2.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([0.0, 55.0], [-7.0, -10.0, -7.0], [14.0, 16.0, 20.0]))
                .cube(cube([0.0, 91.0], [-6.0, 6.0, -7.0], [12.0, 13.0, 18.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-8.0, -13.0, 18.0))
                    .cube(cube([96.0, 0.0], [-4.0, 0.0, -4.0], [8.0, 37.0, 8.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(8.0, -13.0, 18.0)).cube(
                    cube_mirrored([96.0, 0.0], [-4.0, 0.0, -4.0], [8.0, 37.0, 8.0]),
                ),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-8.0, -13.0, -5.0))
                    .cube(cube([64.0, 0.0], [-4.0, 0.0, -4.0], [8.0, 37.0, 8.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(8.0, -13.0, -5.0)).cube(
                    cube_mirrored([64.0, 0.0], [-4.0, 0.0, -4.0], [8.0, 37.0, 8.0]),
                ),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_ravager_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_ravager_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn apply_ravager_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let stunned_tick = state.stunned_ticks_remaining;
    let attack_tick = state.attack_ticks_remaining;
    let attack_time = 10.0_f32;
    let head_anim = if attack_tick > 0.0 {
        (((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25) / ((10.0) * 0.25)
    } else {
        0.0
    };
    let scaled = if attack_tick > 0.0 {
        (1.0 + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25) / ((10.0) * 0.25)))
            * 0.5
    } else {
        0.0
    };
    let head_pos = if attack_tick > 0.0 {
        ((1.0
            + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25) / ((10.0) * 0.25)))
            * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * 12.0
    } else {
        0.0
    };
    let y_offset = if attack_tick > 0.0 {
        (((1.0
            + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25) / ((10.0) * 0.25)))
            * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * 12.0)
            * (tree.part_or_panic("neck").pose.rotation.x).sin()
    } else {
        0.0
    };
    let head_pos = if !(attack_tick > 0.0) { -1.0 } else { 0.0 };
    let y_offset = if !(attack_tick > 0.0) {
        -1.0 * (tree.part_or_panic("neck").pose.rotation.x).sin()
    } else {
        0.0
    };
    let is_stunned = if !(attack_tick > 0.0) {
        stunned_tick > 0.0
    } else {
        false
    };
    let speed = if (!(attack_tick > 0.0)) && (stunned_tick > 0.0) {
        stunned_tick / 40.0
    } else {
        0.0
    };
    let mouth_anim =
        if (!(attack_tick > 0.0)) && (!(stunned_tick > 0.0) && (state.roar_animation > 0.0)) {
            (state.roar_animation * std::f32::consts::PI * 0.25).sin()
        } else {
            0.0
        };
    let animation_pos = state.walk_animation_pos;
    let leg_rot = 0.4 * state.walk_animation_speed;
    let anim_value_0 = if attack_tick > 0.0 {
        -6.5 + (((1.0
            + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25) / ((10.0) * 0.25)))
            * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * 12.0)
    } else {
        tree.part_or_panic("neck").pose.offset.z
    };
    tree.part_mut_or_panic("neck").pose.offset.z = anim_value_0;
    let anim_value_1 = if attack_tick > 0.0 {
        -7.0 - ((((1.0
            + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                / ((10.0) * 0.25)))
            * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * ((1.0
                + ((((attack_tick) % (10.0) - (10.0) * 0.5).abs() - (10.0) * 0.25)
                    / ((10.0) * 0.25)))
                * 0.5)
            * 12.0)
            * (tree.part_or_panic("neck").pose.rotation.x).sin())
    } else {
        tree.part_or_panic("neck").pose.offset.y
    };
    tree.part_mut_or_panic("neck").pose.offset.y = anim_value_1;
    let anim_value_2 = if (attack_tick > 0.0) && (attack_tick > 5.0) {
        ((-4.0 + attack_tick) / 4.0).sin() * std::f32::consts::PI * 0.4
    } else {
        tree.part_or_panic("mouth").pose.rotation.x
    };
    tree.part_mut_or_panic("mouth").pose.rotation.x = anim_value_2;
    let anim_value_3 = if (attack_tick > 0.0) && (!(attack_tick > 5.0)) {
        (std::f32::consts::PI / 20.0_f32) * (std::f32::consts::PI * attack_tick / 10.0).sin()
    } else {
        tree.part_or_panic("mouth").pose.rotation.x
    };
    tree.part_mut_or_panic("mouth").pose.rotation.x = anim_value_3;
    let anim_value_4 = if !(attack_tick > 0.0) {
        0.0
    } else {
        tree.part_or_panic("neck").pose.offset.x
    };
    tree.part_mut_or_panic("neck").pose.offset.x = anim_value_4;
    let anim_value_5 = if !(attack_tick > 0.0) {
        -7.0 - (-1.0 * (tree.part_or_panic("neck").pose.rotation.x).sin())
    } else {
        tree.part_or_panic("neck").pose.offset.y
    };
    tree.part_mut_or_panic("neck").pose.offset.y = anim_value_5;
    let anim_value_6 = if !(attack_tick > 0.0) {
        5.5
    } else {
        tree.part_or_panic("neck").pose.offset.z
    };
    tree.part_mut_or_panic("neck").pose.offset.z = anim_value_6;
    let anim_value_7 = if !(attack_tick > 0.0) {
        if stunned_tick > 0.0 { 0.21991149 } else { 0.0 }
    } else {
        tree.part_or_panic("neck").pose.rotation.x
    };
    tree.part_mut_or_panic("neck").pose.rotation.x = anim_value_7;
    let anim_value_8 = if !(attack_tick > 0.0) {
        std::f32::consts::PI * (if stunned_tick > 0.0 { 0.05 } else { 0.01 })
    } else {
        tree.part_or_panic("mouth").pose.rotation.x
    };
    tree.part_mut_or_panic("mouth").pose.rotation.x = anim_value_8;
    let anim_value_9 = if (!(attack_tick > 0.0)) && (stunned_tick > 0.0) {
        ((stunned_tick / 40.0) * 10.0).sin() * 3.0
    } else {
        tree.part_or_panic("neck").pose.offset.x
    };
    tree.part_mut_or_panic("neck").pose.offset.x = anim_value_9;
    let anim_value_10 =
        if (!(attack_tick > 0.0)) && (!(stunned_tick > 0.0) && (state.roar_animation > 0.0)) {
            (std::f32::consts::PI / 2.0_f32)
                * ((state.roar_animation * std::f32::consts::PI * 0.25).sin())
        } else {
            tree.part_or_panic("mouth").pose.rotation.x
        };
    tree.part_mut_or_panic("mouth").pose.rotation.x = anim_value_10;
    let anim_value_11 = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.x = anim_value_11;
    let anim_value_12 = state.y_rot_degrees * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head").pose.rotation.y = anim_value_12;
    let anim_value_13 = (animation_pos * 0.6662).cos() * leg_rot;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_13;
    let anim_value_14 = (animation_pos * 0.6662 + std::f32::consts::PI).cos() * leg_rot;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_14;
    let anim_value_15 = (animation_pos * 0.6662 + std::f32::consts::PI).cos() * leg_rot;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_15;
    let anim_value_16 = (animation_pos * 0.6662).cos() * leg_rot;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_16;
}
pub fn apply_ravager_ravager_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_ravager_model_setup_anim_ir(tree, state);
}
pub fn build_ravager_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_ravager_model_tree();
    apply_ravager_ravager_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 128.0,
    })
}
pub fn build_turtle_model_tree() -> JavaModelPartTree {
    let root =
        NamedPartSpec::new("root", PartPose::ZERO)
            .child(
                NamedPartSpec::new("head", PartPose::offset(0.0, 19.0, -10.0)).cube(cube(
                    [3.0, 0.0],
                    [-3.0, -1.0, -3.0],
                    [6.0, 5.0, 6.0],
                )),
            )
            .child(
                NamedPartSpec::new(
                    "body",
                    PartPose::offset_and_rotation(0.0, 11.0, -10.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([7.0, 37.0], [-9.5, 3.0, -10.0], [19.0, 20.0, 6.0]))
                .cube(cube([31.0, 1.0], [-5.5, 3.0, -13.0], [11.0, 18.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "egg_belly",
                    PartPose::offset_and_rotation(0.0, 11.0, -10.0, 1.570796, 0.0, 0.0),
                )
                .cube(cube([70.0, 33.0], [-4.5, 3.0, -14.0], [9.0, 18.0, 1.0])),
            )
            .child(
                NamedPartSpec::new("right_hind_leg", PartPose::offset(-3.5, 22.0, 11.0))
                    .cube(cube([1.0, 23.0], [-2.0, 0.0, 0.0], [4.0, 1.0, 10.0])),
            )
            .child(
                NamedPartSpec::new("left_hind_leg", PartPose::offset(3.5, 22.0, 11.0)).cube(cube(
                    [1.0, 12.0],
                    [-2.0, 0.0, 0.0],
                    [4.0, 1.0, 10.0],
                )),
            )
            .child(
                NamedPartSpec::new("right_front_leg", PartPose::offset(-5.0, 21.0, -4.0))
                    .cube(cube([27.0, 30.0], [-13.0, 0.0, -2.0], [13.0, 1.0, 5.0])),
            )
            .child(
                NamedPartSpec::new("left_front_leg", PartPose::offset(5.0, 21.0, -4.0)).cube(cube(
                    [27.0, 24.0],
                    [0.0, 0.0, -2.0],
                    [13.0, 1.0, 5.0],
                )),
            );
    JavaModelPartTree::from_root(root)
}
pub fn build_turtle_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_turtle_model_tree().mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn apply_turtle_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let animation_pos = state.walk_animation_pos;
    let animation_speed = state.walk_animation_speed;
    let lay_egg = if state.is_on_land {
        if state.is_laying_egg { 4.0 } else { 1.0 }
    } else {
        0.0
    };
    let lay_egg_amplitude = if state.is_on_land {
        if state.is_laying_egg { 2.0 } else { 1.0 }
    } else {
        0.0
    };
    let swing_pos = if state.is_on_land {
        animation_pos * 5.0
    } else {
        0.0
    };
    let front_swing = if state.is_on_land {
        ((if state.is_laying_egg { 4.0 } else { 1.0 }) * (animation_pos * 5.0)).cos()
    } else {
        0.0
    };
    let hind_swing = if state.is_on_land {
        (animation_pos * 5.0).cos()
    } else {
        0.0
    };
    let swing_scale = if !(state.is_on_land) {
        0.5 * animation_speed
    } else {
        0.0
    };
    let swing = if !(state.is_on_land) {
        (animation_pos * 0.6662 * 0.6).cos() * (0.5 * animation_speed)
    } else {
        0.0
    };
    let anim_value_0 = if state.is_on_land {
        -(((if state.is_laying_egg { 4.0 } else { 1.0 }) * (animation_pos * 5.0)).cos())
            * 8.0
            * animation_speed
            * (if state.is_laying_egg { 2.0 } else { 1.0 })
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.y = anim_value_0;
    let anim_value_1 = if state.is_on_land {
        (((if state.is_laying_egg { 4.0 } else { 1.0 }) * (animation_pos * 5.0)).cos())
            * 8.0
            * animation_speed
            * (if state.is_laying_egg { 2.0 } else { 1.0 })
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.y = anim_value_1;
    let anim_value_2 = if state.is_on_land {
        -((animation_pos * 5.0).cos()) * 3.0 * animation_speed
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.y = anim_value_2;
    let anim_value_3 = if state.is_on_land {
        ((animation_pos * 5.0).cos()) * 3.0 * animation_speed
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.y
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.y = anim_value_3;
    let anim_value_4 = if !(state.is_on_land) {
        (animation_pos * 0.6662 * 0.6).cos() * (0.5 * animation_speed)
    } else {
        tree.part_or_panic("right_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_4;
    let anim_value_5 = if !(state.is_on_land) {
        -((animation_pos * 0.6662 * 0.6).cos() * (0.5 * animation_speed))
    } else {
        tree.part_or_panic("left_hind_leg").pose.rotation.x
    };
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_5;
    let anim_value_6 = if !(state.is_on_land) {
        -((animation_pos * 0.6662 * 0.6).cos() * (0.5 * animation_speed))
    } else {
        tree.part_or_panic("right_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("right_front_leg").pose.rotation.z = anim_value_6;
    let anim_value_7 = if !(state.is_on_land) {
        (animation_pos * 0.6662 * 0.6).cos() * (0.5 * animation_speed)
    } else {
        tree.part_or_panic("left_front_leg").pose.rotation.z
    };
    tree.part_mut_or_panic("left_front_leg").pose.rotation.z = anim_value_7;
}
pub fn apply_adult_turtle_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.has_egg;
    tree.part_mut_or_panic("egg_belly").visible = anim_value_0;
    let anim_value_1 = if tree.part_or_panic("egg_belly").visible {
        tree.part_or_panic("root").pose.offset.y - (1.0)
    } else {
        tree.part_or_panic("root").pose.offset.y
    };
    tree.part_mut_or_panic("root").pose.offset.y = anim_value_1;
}
pub fn apply_turtle_adult_turtle_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_quadruped_model_setup_anim_ir(tree, state);
    apply_turtle_model_setup_anim_ir(tree, state);
    apply_adult_turtle_model_setup_anim_ir(tree, state);
}
pub fn build_turtle_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_turtle_model_tree();
    apply_turtle_adult_turtle_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 128.0,
        height: 64.0,
    })
}
pub fn build_donkey_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new(
        "root",
        PartPose {
            offset: Vec3::new(0.0, 3.12208, 0.0),
            rotation: Vec3::splat(0.0),
            scale: Vec3::splat(0.87),
        },
    )
    .child(
        NamedPartSpec::new("body", PartPose::offset(0.0, 11.0, 5.0))
            .cube(cube_grown(
                [0.0, 32.0],
                [-5.0, -8.0, -17.0],
                [10.0, 10.0, 22.0],
                0.05,
            ))
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(0.0, -5.0, 2.0, 0.523599, 0.0, 0.0),
                )
                .cube(cube([42.0, 36.0], [-1.5, 0.0, 0.0], [3.0, 14.0, 4.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_chest",
                    PartPose::offset_and_rotation(
                        6.0,
                        -8.0,
                        0.0,
                        0.0,
                        -std::f32::consts::FRAC_PI_2,
                        0.0,
                    ),
                )
                .visible(false)
                .cube(cube([26.0, 21.0], [-4.0, 0.0, -2.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_chest",
                    PartPose::offset_and_rotation(
                        -6.0,
                        -8.0,
                        0.0,
                        0.0,
                        std::f32::consts::FRAC_PI_2,
                        0.0,
                    ),
                )
                .visible(false)
                .cube(cube([26.0, 21.0], [-4.0, 0.0, -2.0], [8.0, 8.0, 3.0])),
            ),
    )
    .child(
        NamedPartSpec::new(
            "head_parts",
            PartPose::offset_and_rotation(0.0, 4.0, -12.0, 0.523599, 0.0, 0.0),
        )
        .cube(cube([0.0, 35.0], [-2.05, -6.0, -2.0], [4.0, 12.0, 7.0]))
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 13.0], [-3.0, -11.0, -2.0], [6.0, 5.0, 7.0]))
                .child(
                    NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube_grown(
                        [19.0, 16.0],
                        [0.55, -13.0, 4.0],
                        [2.0, 3.0, 1.0],
                        -0.001,
                    )),
                )
                .child(
                    NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube_grown(
                        [19.0, 16.0],
                        [-2.55, -13.0, 4.0],
                        [2.0, 3.0, 1.0],
                        -0.001,
                    )),
                ),
        )
        .child(NamedPartSpec::new("mane", PartPose::ZERO).cube(cube(
            [56.0, 36.0],
            [-1.0, -11.0, 5.01],
            [2.0, 16.0, 2.0],
        )))
        .child(NamedPartSpec::new("upper_mouth", PartPose::ZERO).cube(cube(
            [0.0, 25.0],
            [-2.0, -11.0, -7.0],
            [4.0, 5.0, 5.0],
        ))),
    )
    .child(
        NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 14.0, 7.0)).cube(cube_mirrored(
            [48.0, 21.0],
            [-3.0, -1.01, -1.0],
            [4.0, 11.0, 4.0],
        )),
    )
    .child(
        NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 14.0, 7.0)).cube(cube(
            [48.0, 21.0],
            [-1.0, -1.01, -1.0],
            [4.0, 11.0, 4.0],
        )),
    )
    .child(
        NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 14.0, -10.0)).cube(
            cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.9], [4.0, 11.0, 4.0]),
        ),
    )
    .child(
        NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 14.0, -10.0)).cube(cube(
            [48.0, 21.0],
            [-1.0, -1.01, -1.9],
            [4.0, 11.0, 4.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_donkey_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_donkey_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_abstract_equine_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let clamped_y_rot = (state.y_rot_degrees).clamp(-20.0, 20.0);
    let mut head_rot_x_rad = state.x_rot_degrees * (std::f32::consts::PI / 180.0);
    let animation_speed = state.walk_animation_speed;
    let animation_pos = state.walk_animation_pos;
    if animation_speed > 0.2 {
        head_rot_x_rad += (animation_pos * 0.8).cos() * 0.15 * animation_speed;
    }
    let eating = state.eat_animation;
    let standing = state.stand_animation;
    let i_standing = 1.0 - standing;
    let feeding_anim = state.feeding_animation;
    let animate_tail = state.animate_tail;
    let water_multiplier = if state.is_in_water { 0.2 } else { 1.0 };
    let leg_anim1 = (water_multiplier * animation_pos * 0.6662 + std::f32::consts::PI).cos();
    let leg_x_rot_anim = leg_anim1 * 0.8 * animation_speed;
    let base_head_angle = (1.0 - (standing).max(eating))
        * ((std::f32::consts::PI / 6.0_f32)
            + head_rot_x_rad
            + feeding_anim * (state.age_in_ticks).sin() * 0.05);
    let stand_angle = state.leg_stand_angle * standing;
    let bob_value = (state.age_in_ticks * 0.6 + std::f32::consts::PI).cos();
    let leg_standing_x_rot_offset = state.leg_standing_x_rot_offset;
    let rleg_rot = (leg_standing_x_rot_offset + bob_value) * standing + leg_x_rot_anim * i_standing;
    let lleg_rot = (leg_standing_x_rot_offset - bob_value) * standing - leg_x_rot_anim * i_standing;
    let age_scale = state.age_scale;
    let anim_value_0 = (std::f32::consts::PI / 6.0_f32) + head_rot_x_rad;
    tree.part_mut_or_panic("head_parts").pose.rotation.x = anim_value_0;
    let anim_value_1 = clamped_y_rot * (std::f32::consts::PI / 180.0);
    tree.part_mut_or_panic("head_parts").pose.rotation.y = anim_value_1;
    let anim_value_2 = standing * ((std::f32::consts::PI / 12.0_f32) + head_rot_x_rad)
        + eating * (2.1816616 + (state.age_in_ticks).sin() * 0.05)
        + base_head_angle;
    tree.part_mut_or_panic("head_parts").pose.rotation.x = anim_value_2;
    let anim_value_3 = standing * clamped_y_rot * (std::f32::consts::PI / 180.0)
        + (1.0 - (standing).max(eating)) * tree.part_or_panic("head_parts").pose.rotation.y;
    tree.part_mut_or_panic("head_parts").pose.rotation.y = anim_value_3;
    let anim_value_4 = tree.part_or_panic("head_parts").pose.offset.y
        + ((0.0 + (standing) * (-8.0 - 0.0))
            + (eating) * (7.0 - (0.0 + (standing) * (-8.0 - 0.0))));
    tree.part_mut_or_panic("head_parts").pose.offset.y = anim_value_4;
    let anim_value_5 = tree.part_or_panic("head_parts").pose.offset.z
        + (standing) * (-4.0 - tree.part_or_panic("head_parts").pose.offset.z);
    tree.part_mut_or_panic("head_parts").pose.offset.z = anim_value_5;
    let anim_value_6 = standing * (-std::f32::consts::PI / 4.0_f32)
        + i_standing * tree.part_or_panic("body").pose.rotation.x;
    tree.part_mut_or_panic("body").pose.rotation.x = anim_value_6;
    let anim_value_7 =
        tree.part_or_panic("left_front_leg").pose.offset.y - state.leg_standing_y_offset * standing;
    tree.part_mut_or_panic("left_front_leg").pose.offset.y = anim_value_7;
    let anim_value_8 =
        tree.part_or_panic("left_front_leg").pose.offset.z + state.leg_standing_z_offset * standing;
    tree.part_mut_or_panic("left_front_leg").pose.offset.z = anim_value_8;
    let anim_value_9 = tree.part_or_panic("left_front_leg").pose.offset.y;
    tree.part_mut_or_panic("right_front_leg").pose.offset.y = anim_value_9;
    let anim_value_10 = tree.part_or_panic("left_front_leg").pose.offset.z;
    tree.part_mut_or_panic("right_front_leg").pose.offset.z = anim_value_10;
    let anim_value_11 = stand_angle - leg_anim1 * 0.5 * animation_speed * i_standing;
    tree.part_mut_or_panic("left_hind_leg").pose.rotation.x = anim_value_11;
    let anim_value_12 = stand_angle + leg_anim1 * 0.5 * animation_speed * i_standing;
    tree.part_mut_or_panic("right_hind_leg").pose.rotation.x = anim_value_12;
    let anim_value_13 = rleg_rot;
    tree.part_mut_or_panic("left_front_leg").pose.rotation.x = anim_value_13;
    let anim_value_14 = lleg_rot;
    tree.part_mut_or_panic("right_front_leg").pose.rotation.x = anim_value_14;
    let anim_value_15 =
        state.tail_x_rot_offset + (std::f32::consts::PI / 6.0_f32) + animation_speed * 0.75;
    tree.part_mut_or_panic("tail").pose.rotation.x = anim_value_15;
    let anim_value_16 = animation_speed * age_scale;
    tree.part_mut_or_panic("tail").pose.offset.y += anim_value_16;
    let anim_value_17 = animation_speed * 2.0 * age_scale;
    tree.part_mut_or_panic("tail").pose.offset.z += anim_value_17;
    let anim_value_18 = if animate_tail {
        (state.age_in_ticks * 0.7).cos()
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_18;
    let anim_value_19 = if !(animate_tail) {
        0.0
    } else {
        tree.part_or_panic("tail").pose.rotation.y
    };
    tree.part_mut_or_panic("tail").pose.rotation.y = anim_value_19;
}
pub fn apply_donkey_model_setup_anim_ir(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    let anim_value_0 = state.has_chest;
    tree.part_mut_or_panic("left_chest").visible = anim_value_0;
    let anim_value_1 = state.has_chest;
    tree.part_mut_or_panic("right_chest").visible = anim_value_1;
}
pub fn apply_donkey_donkey_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_equine_model_setup_anim_ir(tree, state);
    apply_donkey_model_setup_anim_ir(tree, state);
}
pub fn build_donkey_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_donkey_model_tree();
    apply_donkey_donkey_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_horse_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 11.0, 5.0))
                .cube(cube_grown(
                    [0.0, 32.0],
                    [-5.0, -8.0, -17.0],
                    [10.0, 10.0, 22.0],
                    0.05,
                ))
                .child(
                    NamedPartSpec::new(
                        "tail",
                        PartPose::offset_and_rotation(0.0, -5.0, 2.0, 0.523599, 0.0, 0.0),
                    )
                    .cube(cube(
                        [42.0, 36.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 14.0, 4.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "head_parts",
                PartPose::offset_and_rotation(0.0, 4.0, -12.0, 0.523599, 0.0, 0.0),
            )
            .cube(cube([0.0, 35.0], [-2.05, -6.0, -2.0], [4.0, 12.0, 7.0]))
            .child(
                NamedPartSpec::new("head", PartPose::ZERO)
                    .cube(cube([0.0, 13.0], [-3.0, -11.0, -2.0], [6.0, 5.0, 7.0]))
                    .child(
                        NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [0.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    )
                    .child(
                        NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [-2.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    ),
            )
            .child(NamedPartSpec::new("mane", PartPose::ZERO).cube(cube(
                [56.0, 36.0],
                [-1.0, -11.0, 5.01],
                [2.0, 16.0, 2.0],
            )))
            .child(NamedPartSpec::new("upper_mouth", PartPose::ZERO).cube(cube(
                [0.0, 25.0],
                [-2.0, -11.0, -7.0],
                [4.0, 5.0, 5.0],
            ))),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 14.0, 7.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.0], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 14.0, 7.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.0],
                [4.0, 11.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 14.0, -10.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.9], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 14.0, -10.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.9],
                [4.0, 11.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_horse_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_horse_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_horse_abstract_equine_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_equine_model_setup_anim_ir(tree, state);
}
pub fn build_horse_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_horse_model_tree();
    apply_horse_abstract_equine_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_mule_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new(
        "root",
        PartPose {
            offset: Vec3::new(0.0, 1.92, 0.0),
            rotation: Vec3::splat(0.0),
            scale: Vec3::splat(0.92),
        },
    )
    .child(
        NamedPartSpec::new("body", PartPose::offset(0.0, 11.0, 5.0))
            .cube(cube_grown(
                [0.0, 32.0],
                [-5.0, -8.0, -17.0],
                [10.0, 10.0, 22.0],
                0.05,
            ))
            .child(
                NamedPartSpec::new(
                    "tail",
                    PartPose::offset_and_rotation(0.0, -5.0, 2.0, 0.523599, 0.0, 0.0),
                )
                .cube(cube([42.0, 36.0], [-1.5, 0.0, 0.0], [3.0, 14.0, 4.0])),
            )
            .child(
                NamedPartSpec::new(
                    "left_chest",
                    PartPose::offset_and_rotation(
                        6.0,
                        -8.0,
                        0.0,
                        0.0,
                        -std::f32::consts::FRAC_PI_2,
                        0.0,
                    ),
                )
                .visible(false)
                .cube(cube([26.0, 21.0], [-4.0, 0.0, -2.0], [8.0, 8.0, 3.0])),
            )
            .child(
                NamedPartSpec::new(
                    "right_chest",
                    PartPose::offset_and_rotation(
                        -6.0,
                        -8.0,
                        0.0,
                        0.0,
                        std::f32::consts::FRAC_PI_2,
                        0.0,
                    ),
                )
                .visible(false)
                .cube(cube([26.0, 21.0], [-4.0, 0.0, -2.0], [8.0, 8.0, 3.0])),
            ),
    )
    .child(
        NamedPartSpec::new(
            "head_parts",
            PartPose::offset_and_rotation(0.0, 4.0, -12.0, 0.523599, 0.0, 0.0),
        )
        .cube(cube([0.0, 35.0], [-2.05, -6.0, -2.0], [4.0, 12.0, 7.0]))
        .child(
            NamedPartSpec::new("head", PartPose::ZERO)
                .cube(cube([0.0, 13.0], [-3.0, -11.0, -2.0], [6.0, 5.0, 7.0]))
                .child(
                    NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube_grown(
                        [19.0, 16.0],
                        [0.55, -13.0, 4.0],
                        [2.0, 3.0, 1.0],
                        -0.001,
                    )),
                )
                .child(
                    NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube_grown(
                        [19.0, 16.0],
                        [-2.55, -13.0, 4.0],
                        [2.0, 3.0, 1.0],
                        -0.001,
                    )),
                ),
        )
        .child(NamedPartSpec::new("mane", PartPose::ZERO).cube(cube(
            [56.0, 36.0],
            [-1.0, -11.0, 5.01],
            [2.0, 16.0, 2.0],
        )))
        .child(NamedPartSpec::new("upper_mouth", PartPose::ZERO).cube(cube(
            [0.0, 25.0],
            [-2.0, -11.0, -7.0],
            [4.0, 5.0, 5.0],
        ))),
    )
    .child(
        NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 14.0, 7.0)).cube(cube_mirrored(
            [48.0, 21.0],
            [-3.0, -1.01, -1.0],
            [4.0, 11.0, 4.0],
        )),
    )
    .child(
        NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 14.0, 7.0)).cube(cube(
            [48.0, 21.0],
            [-1.0, -1.01, -1.0],
            [4.0, 11.0, 4.0],
        )),
    )
    .child(
        NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 14.0, -10.0)).cube(
            cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.9], [4.0, 11.0, 4.0]),
        ),
    )
    .child(
        NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 14.0, -10.0)).cube(cube(
            [48.0, 21.0],
            [-1.0, -1.01, -1.9],
            [4.0, 11.0, 4.0],
        )),
    );
    JavaModelPartTree::from_root(root)
}
pub fn build_mule_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_mule_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_mule_donkey_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_equine_model_setup_anim_ir(tree, state);
    apply_donkey_model_setup_anim_ir(tree, state);
}
pub fn build_mule_mesh_animated(state: EntityModelAnimationState) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_mule_model_tree();
    apply_mule_donkey_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_skeleton_horse_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 11.0, 5.0))
                .cube(cube_grown(
                    [0.0, 32.0],
                    [-5.0, -8.0, -17.0],
                    [10.0, 10.0, 22.0],
                    0.05,
                ))
                .child(
                    NamedPartSpec::new(
                        "tail",
                        PartPose::offset_and_rotation(0.0, -5.0, 2.0, 0.523599, 0.0, 0.0),
                    )
                    .cube(cube(
                        [42.0, 36.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 14.0, 4.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "head_parts",
                PartPose::offset_and_rotation(0.0, 4.0, -12.0, 0.523599, 0.0, 0.0),
            )
            .cube(cube([0.0, 35.0], [-2.05, -6.0, -2.0], [4.0, 12.0, 7.0]))
            .child(
                NamedPartSpec::new("head", PartPose::ZERO)
                    .cube(cube([0.0, 13.0], [-3.0, -11.0, -2.0], [6.0, 5.0, 7.0]))
                    .child(
                        NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [0.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    )
                    .child(
                        NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [-2.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    ),
            )
            .child(NamedPartSpec::new("mane", PartPose::ZERO).cube(cube(
                [56.0, 36.0],
                [-1.0, -11.0, 5.01],
                [2.0, 16.0, 2.0],
            )))
            .child(NamedPartSpec::new("upper_mouth", PartPose::ZERO).cube(cube(
                [0.0, 25.0],
                [-2.0, -11.0, -7.0],
                [4.0, 5.0, 5.0],
            ))),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 14.0, 7.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.0], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 14.0, 7.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.0],
                [4.0, 11.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 14.0, -10.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.9], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 14.0, -10.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.9],
                [4.0, 11.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_skeleton_horse_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_skeleton_horse_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_skeleton_horse_abstract_equine_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_equine_model_setup_anim_ir(tree, state);
}
pub fn build_skeleton_horse_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_skeleton_horse_model_tree();
    apply_skeleton_horse_abstract_equine_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn build_zombie_horse_model_tree() -> JavaModelPartTree {
    let root = NamedPartSpec::new("root", PartPose::ZERO)
        .child(
            NamedPartSpec::new("body", PartPose::offset(0.0, 11.0, 5.0))
                .cube(cube_grown(
                    [0.0, 32.0],
                    [-5.0, -8.0, -17.0],
                    [10.0, 10.0, 22.0],
                    0.05,
                ))
                .child(
                    NamedPartSpec::new(
                        "tail",
                        PartPose::offset_and_rotation(0.0, -5.0, 2.0, 0.523599, 0.0, 0.0),
                    )
                    .cube(cube(
                        [42.0, 36.0],
                        [-1.5, 0.0, 0.0],
                        [3.0, 14.0, 4.0],
                    )),
                ),
        )
        .child(
            NamedPartSpec::new(
                "head_parts",
                PartPose::offset_and_rotation(0.0, 4.0, -12.0, 0.523599, 0.0, 0.0),
            )
            .cube(cube([0.0, 35.0], [-2.05, -6.0, -2.0], [4.0, 12.0, 7.0]))
            .child(
                NamedPartSpec::new("head", PartPose::ZERO)
                    .cube(cube([0.0, 13.0], [-3.0, -11.0, -2.0], [6.0, 5.0, 7.0]))
                    .child(
                        NamedPartSpec::new("left_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [0.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    )
                    .child(
                        NamedPartSpec::new("right_ear", PartPose::ZERO).cube(cube_grown(
                            [19.0, 16.0],
                            [-2.55, -13.0, 4.0],
                            [2.0, 3.0, 1.0],
                            -0.001,
                        )),
                    ),
            )
            .child(NamedPartSpec::new("mane", PartPose::ZERO).cube(cube(
                [56.0, 36.0],
                [-1.0, -11.0, 5.01],
                [2.0, 16.0, 2.0],
            )))
            .child(NamedPartSpec::new("upper_mouth", PartPose::ZERO).cube(cube(
                [0.0, 25.0],
                [-2.0, -11.0, -7.0],
                [4.0, 5.0, 5.0],
            ))),
        )
        .child(
            NamedPartSpec::new("left_hind_leg", PartPose::offset(4.0, 14.0, 7.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.0], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_hind_leg", PartPose::offset(-4.0, 14.0, 7.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.0],
                [4.0, 11.0, 4.0],
            )),
        )
        .child(
            NamedPartSpec::new("left_front_leg", PartPose::offset(4.0, 14.0, -10.0)).cube(
                cube_mirrored([48.0, 21.0], [-3.0, -1.01, -1.9], [4.0, 11.0, 4.0]),
            ),
        )
        .child(
            NamedPartSpec::new("right_front_leg", PartPose::offset(-4.0, 14.0, -10.0)).cube(cube(
                [48.0, 21.0],
                [-1.0, -1.01, -1.9],
                [4.0, 11.0, 4.0],
            )),
        );
    JavaModelPartTree::from_root(root)
}
pub fn build_zombie_horse_mesh() -> (Vec<Vertex>, Vec<u32>) {
    build_zombie_horse_model_tree().mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
pub fn apply_zombie_horse_abstract_equine_model_setup_anim_chain(
    tree: &mut JavaModelPartTree,
    state: EntityModelAnimationState,
) {
    tree.reset_pose();
    apply_abstract_equine_model_setup_anim_ir(tree, state);
}
pub fn build_zombie_horse_mesh_animated(
    state: EntityModelAnimationState,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut tree = build_zombie_horse_model_tree();
    apply_zombie_horse_abstract_equine_model_setup_anim_chain(&mut tree, state);
    tree.mesh(ModelTextureSize {
        width: 64.0,
        height: 64.0,
    })
}
