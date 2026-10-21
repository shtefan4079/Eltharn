#![allow(dead_code)]
use crate::render::baked_model::Vertex;
use el_math::Direction;
use glam::{Mat4, Vec3};
use std::collections::HashMap;
#[derive(Debug, Clone, Copy)]
pub struct PartPose {
    pub offset: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}
impl PartPose {
    pub const ZERO: Self = Self {
        offset: Vec3::ZERO,
        rotation: Vec3::ZERO,
        scale: Vec3::ONE,
    };
    pub fn offset(x: f32, y: f32, z: f32) -> Self {
        Self {
            offset: Vec3::new(x, y, z),
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
    pub fn rotation(x_rot: f32, y_rot: f32, z_rot: f32) -> Self {
        Self {
            offset: Vec3::ZERO,
            rotation: Vec3::new(x_rot, y_rot, z_rot),
            scale: Vec3::ONE,
        }
    }
    pub fn offset_and_rotation(x: f32, y: f32, z: f32, x_rot: f32, y_rot: f32, z_rot: f32) -> Self {
        Self {
            offset: Vec3::new(x, y, z),
            rotation: Vec3::new(x_rot, y_rot, z_rot),
            scale: Vec3::ONE,
        }
    }
    pub fn translated(self, x: f32, y: f32, z: f32) -> Self {
        Self {
            offset: self.offset + Vec3::new(x, y, z),
            ..self
        }
    }
    pub fn scaled(self, factor: f32) -> Self {
        Self {
            offset: self.offset * factor,
            scale: self.scale * factor,
            ..self
        }
    }
    pub fn matrix(self) -> Mat4 {
        Mat4::from_translation(self.offset)
            * Mat4::from_rotation_z(self.rotation.z)
            * Mat4::from_rotation_y(self.rotation.y)
            * Mat4::from_rotation_x(self.rotation.x)
            * Mat4::from_scale(self.scale)
    }
}
#[derive(Debug, Clone)]
pub struct CubeSpec {
    pub tex_offs: [f32; 2],
    pub origin: Vec3,
    pub dimensions: Vec3,
    pub grow: Vec3,
    pub mirror: bool,
}
impl CubeSpec {
    pub fn new(
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
#[derive(Debug, Clone)]
pub struct PartSpec {
    pub pose: PartPose,
    pub cubes: Vec<CubeSpec>,
    pub children: Vec<PartSpec>,
}
impl PartSpec {
    pub fn new(pose: PartPose) -> Self {
        Self {
            pose,
            cubes: Vec::new(),
            children: Vec::new(),
        }
    }
    pub fn cube(mut self, cube: CubeSpec) -> Self {
        self.cubes.push(cube);
        self
    }
    pub fn child(mut self, child: PartSpec) -> Self {
        self.children.push(child);
        self
    }
}
#[derive(Debug, Clone)]
pub struct NamedPartSpec {
    pub name: &'static str,
    pub pose: PartPose,
    pub cubes: Vec<CubeSpec>,
    pub children: Vec<NamedPartSpec>,
    pub visible: bool,
    pub skip_draw: bool,
}
impl NamedPartSpec {
    pub fn new(name: &'static str, pose: PartPose) -> Self {
        Self {
            name,
            pose,
            cubes: Vec::new(),
            children: Vec::new(),
            visible: true,
            skip_draw: false,
        }
    }
    pub fn cube(mut self, cube: CubeSpec) -> Self {
        self.cubes.push(cube);
        self
    }
    pub fn child(mut self, child: NamedPartSpec) -> Self {
        self.children.push(child);
        self
    }
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    pub fn skip_draw(mut self, skip_draw: bool) -> Self {
        self.skip_draw = skip_draw;
        self
    }
}
#[derive(Debug, Clone)]
pub struct JavaModelPartNode {
    pub name: &'static str,
    pub pose: PartPose,
    pub initial_pose: PartPose,
    pub cubes: Vec<CubeSpec>,
    pub children: Vec<usize>,
    pub visible: bool,
    pub skip_draw: bool,
}
impl JavaModelPartNode {
    pub fn load_pose(&mut self, pose: PartPose) {
        self.pose = pose;
    }
    pub fn reset_pose(&mut self) {
        self.load_pose(self.initial_pose);
    }
    pub fn set_pos(&mut self, x: f32, y: f32, z: f32) {
        self.pose.offset = Vec3::new(x, y, z);
    }
    pub fn set_rotation(&mut self, x_rot: f32, y_rot: f32, z_rot: f32) {
        self.pose.rotation = Vec3::new(x_rot, y_rot, z_rot);
    }
    pub fn offset_pos(&mut self, offset: Vec3) {
        self.pose.offset += offset;
    }
    pub fn offset_rotation(&mut self, offset: Vec3) {
        self.pose.rotation += offset;
    }
    pub fn offset_scale(&mut self, offset: Vec3) {
        self.pose.scale += offset;
    }
}
#[derive(Debug, Clone)]
pub struct JavaModelPartTree {
    pub nodes: Vec<JavaModelPartNode>,
    pub lookup: HashMap<&'static str, usize>,
}
impl JavaModelPartTree {
    pub fn from_root(root: NamedPartSpec) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            lookup: HashMap::new(),
        };
        tree.push_part(root);
        tree
    }
    pub fn push_part(&mut self, spec: NamedPartSpec) -> usize {
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
    pub fn part(&self, name: &str) -> Option<&JavaModelPartNode> {
        self.lookup
            .get(name)
            .and_then(|index| self.nodes.get(*index))
    }
    pub fn part_or_panic(&self, name: &str) -> &JavaModelPartNode {
        self.part(name)
            .unwrap_or_else(|| panic!("Can't find part {name}"))
    }
    pub fn part_mut(&mut self, name: &str) -> Option<&mut JavaModelPartNode> {
        let index = *self.lookup.get(name)?;
        self.nodes.get_mut(index)
    }
    pub fn part_mut_or_panic(&mut self, name: &str) -> &mut JavaModelPartNode {
        self.part_mut(name)
            .unwrap_or_else(|| panic!("Can't find part {name}"))
    }
    pub fn reset_pose(&mut self) {
        for node in &mut self.nodes {
            node.reset_pose();
        }
    }
    pub fn mesh(&self, texture: ModelTextureSize) -> (Vec<Vertex>, Vec<u32>) {
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
pub fn cube(tex_offs: [f32; 2], origin: [f32; 3], dimensions: [f32; 3]) -> CubeSpec {
    CubeSpec::new(
        tex_offs,
        Vec3::from(origin),
        Vec3::from(dimensions),
        Vec3::ZERO,
        false,
    )
}
pub fn cube_grown(
    tex_offs: [f32; 2],
    origin: [f32; 3],
    dimensions: [f32; 3],
    grow: f32,
) -> CubeSpec {
    CubeSpec::new(
        tex_offs,
        Vec3::from(origin),
        Vec3::from(dimensions),
        Vec3::splat(grow),
        false,
    )
}
pub fn cube_mirrored(tex_offs: [f32; 2], origin: [f32; 3], dimensions: [f32; 3]) -> CubeSpec {
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
fn part_normal_matrix(matrix: Mat4) -> Option<Mat4> {
    let determinant = matrix.determinant();
    if !matrix.is_finite() || !determinant.is_finite() || determinant == 0.0 {
        return None;
    }
    let normal = matrix.inverse().transpose();
    normal.is_finite().then_some(normal)
}
fn collect_part_vertices(
    part: &PartSpec,
    parent_matrix: Mat4,
    texture: ModelTextureSize,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
) {
    let part_matrix = parent_matrix * part.pose.matrix();
    let Some(normal_matrix) = part_normal_matrix(part_matrix) else {
        return;
    };
    for cube in &part.cubes {
        for polygon in cube_polygons(cube, texture) {
            let start = vertices.len() as u32;
            let normal = java_normal_to_world(
                normal_matrix
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
    let Some(normal_matrix) = part_normal_matrix(part_matrix) else {
        return;
    };
    if !node.skip_draw {
        for cube in &node.cubes {
            for polygon in cube_polygons(cube, texture) {
                let start = vertices.len() as u32;
                let normal = java_normal_to_world(
                    normal_matrix
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
pub fn java_pos_to_world(pos: Vec3) -> Vec3 {
    Vec3::new(pos.x, 24.0 - pos.y, -pos.z) / 16.0
}
pub fn java_normal_to_world(normal: Vec3) -> Vec3 {
    Vec3::new(normal.x, -normal.y, -normal.z).normalize_or_zero()
}
#[derive(Debug, Clone, Copy)]
pub struct ModelVertex {
    pub pos: Vec3,
    pub uv: [f32; 2],
}
#[derive(Debug, Clone, Copy)]
pub struct ModelPolygon {
    pub vertices: [ModelVertex; 4],
    pub normal: Vec3,
}
pub fn cube_polygons(cube: &CubeSpec, texture: ModelTextureSize) -> [ModelPolygon; 6] {
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
pub fn direction_normal(direction: Direction) -> Vec3 {
    match direction {
        Direction::Down => Vec3::NEG_Y,
        Direction::Up => Vec3::Y,
        Direction::North => Vec3::NEG_Z,
        Direction::South => Vec3::Z,
        Direction::West => Vec3::NEG_X,
        Direction::East => Vec3::X,
    }
}
