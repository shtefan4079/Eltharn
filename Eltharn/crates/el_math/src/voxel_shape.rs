use crate::aabb::AABB;
use crate::{Axis, BlockPos, Vec3};
pub const EPSILON: f64 = 1.0e-7;
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelShape {
    boxes: Vec<AABB>,
    coordinates: Option<std::sync::Arc<[Vec<f64>; 3]>>,
}
impl VoxelShape {
    pub fn empty() -> Self {
        VoxelShape {
            boxes: Vec::new(),
            coordinates: None,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }
    pub fn cube() -> Self {
        VoxelShape {
            boxes: vec![AABB::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0)],
            coordinates: None,
        }
    }
    pub fn from_aabb(aabb: AABB) -> Self {
        VoxelShape {
            boxes: vec![aabb],
            coordinates: None,
        }
    }
    pub fn from_aabbs(aabbs: Vec<AABB>) -> Self {
        VoxelShape {
            boxes: aabbs,
            coordinates: None,
        }
    }
    pub fn with_coordinates(mut self, coordinates: [Vec<f64>; 3]) -> Self {
        self.coordinates = Some(std::sync::Arc::new(coordinates));
        self
    }
    pub fn coordinates(&self, axis: Axis) -> Vec<f64> {
        let index = match axis {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        };
        if let Some(coordinates) = &self.coordinates {
            return coordinates[index].clone();
        }
        let mut result: Vec<_> = self
            .boxes
            .iter()
            .flat_map(|bb| [axis_value(bb, axis, false), axis_value(bb, axis, true)])
            .collect();
        result.sort_by(f64::total_cmp);
        result.dedup();
        result
    }
    pub fn bounds(&self) -> AABB {
        if self.is_empty() {
            return AABB::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut min_z = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut max_z = f64::MIN;
        for b in &self.boxes {
            min_x = min_x.min(b.min_x);
            min_y = min_y.min(b.min_y);
            min_z = min_z.min(b.min_z);
            max_x = max_x.max(b.max_x);
            max_y = max_y.max(b.max_y);
            max_z = max_z.max(b.max_z);
        }
        AABB::new(min_x, min_y, min_z, max_x, max_y, max_z)
    }
    pub fn to_aabbs(&self) -> &[AABB] {
        &self.boxes
    }
    pub fn for_all_edges(&self, mut consumer: impl FnMut(f64, f64, f64, f64, f64, f64)) {
        if self.is_empty() {
            return;
        }
        let mut xs = Vec::with_capacity(self.boxes.len() * 2);
        let mut ys = Vec::with_capacity(self.boxes.len() * 2);
        let mut zs = Vec::with_capacity(self.boxes.len() * 2);
        for bounds in &self.boxes {
            xs.extend([bounds.min_x, bounds.max_x]);
            ys.extend([bounds.min_y, bounds.max_y]);
            zs.extend([bounds.min_z, bounds.max_z]);
        }
        xs.sort_by(f64::total_cmp);
        ys.sort_by(f64::total_cmp);
        zs.sort_by(f64::total_cmp);
        xs.dedup();
        ys.dedup();
        zs.dedup();
        if xs.len() < 2 || ys.len() < 2 || zs.len() < 2 {
            return;
        }
        let is_full = |x: isize, y: isize, z: isize| {
            if x < 0
                || y < 0
                || z < 0
                || x as usize + 1 >= xs.len()
                || y as usize + 1 >= ys.len()
                || z as usize + 1 >= zs.len()
            {
                return false;
            }
            let x = x as usize;
            let y = y as usize;
            let z = z as usize;
            let point = Vec3::new(
                (xs[x] + xs[x + 1]) * 0.5,
                (ys[y] + ys[y + 1]) * 0.5,
                (zs[z] + zs[z + 1]) * 0.5,
            );
            self.boxes.iter().any(|bounds| bounds.contains(point))
        };
        let mut emit_axis = |axis: Axis| {
            let (a_size, b_size, c_size) = match axis {
                Axis::X => (ys.len() - 1, zs.len() - 1, xs.len() - 1),
                Axis::Y => (xs.len() - 1, zs.len() - 1, ys.len() - 1),
                Axis::Z => (xs.len() - 1, ys.len() - 1, zs.len() - 1),
            };
            for a in 0..=a_size {
                for b in 0..=b_size {
                    let mut last_start: Option<usize> = None;
                    for c in 0..=c_size {
                        let mut full_sectors = 0;
                        let mut odd_sectors = 0;
                        for da in 0..=1isize {
                            for db in 0..=1isize {
                                let full = match axis {
                                    Axis::X => is_full(
                                        c as isize,
                                        a as isize + da - 1,
                                        b as isize + db - 1,
                                    ),
                                    Axis::Y => is_full(
                                        a as isize + da - 1,
                                        c as isize,
                                        b as isize + db - 1,
                                    ),
                                    Axis::Z => is_full(
                                        a as isize + da - 1,
                                        b as isize + db - 1,
                                        c as isize,
                                    ),
                                };
                                if full {
                                    full_sectors += 1;
                                    odd_sectors ^= da ^ db;
                                }
                            }
                        }
                        let edge_is_exposed = full_sectors == 1
                            || full_sectors == 3
                            || (full_sectors == 2 && (odd_sectors & 1) == 0);
                        if edge_is_exposed {
                            last_start.get_or_insert(c);
                        } else if let Some(start) = last_start.take() {
                            match axis {
                                Axis::X => consumer(xs[start], ys[a], zs[b], xs[c], ys[a], zs[b]),
                                Axis::Y => consumer(xs[a], ys[start], zs[b], xs[a], ys[c], zs[b]),
                                Axis::Z => consumer(xs[a], ys[b], zs[start], xs[a], ys[b], zs[c]),
                            }
                        }
                    }
                }
            }
        };
        emit_axis(Axis::X);
        emit_axis(Axis::Y);
        emit_axis(Axis::Z);
    }
    pub fn move_by(&self, dx: f64, dy: f64, dz: f64) -> Self {
        let moved_boxes = self.boxes.iter().map(|b| b.move_by(dx, dy, dz)).collect();
        VoxelShape {
            boxes: moved_boxes,
            coordinates: self.coordinates.as_ref().map(|coords| {
                std::sync::Arc::new(std::array::from_fn(|axis| {
                    coords[axis]
                        .iter()
                        .map(|v| v + [dx, dy, dz][axis])
                        .collect()
                }))
            }),
        }
    }
    pub fn clip_location(&self, from: Vec3, to: Vec3, pos: BlockPos) -> Option<Vec3> {
        if self.is_empty() {
            return None;
        }
        let diff = to.subtract(&from);
        if diff.length_sq() < EPSILON {
            return None;
        }
        let test_point = from.add(diff.x * 0.001, diff.y * 0.001, diff.z * 0.001);
        let local_test_point = Vec3::new(
            test_point.x - pos.x as f64,
            test_point.y - pos.y as f64,
            test_point.z - pos.z as f64,
        );
        if self
            .boxes
            .iter()
            .any(|aabb| aabb.contains(local_test_point))
        {
            return Some(test_point);
        }
        AABB::clip_many(
            self.boxes.iter().map(|aabb| aabb.move_by_block(pos)),
            from,
            to,
        )
    }
    pub fn collide(&self, axis: Axis, collision_box: &AABB, max_dist: f64) -> f64 {
        if self.is_empty() {
            return max_dist;
        }
        if max_dist.abs() < EPSILON {
            return 0.0;
        }
        if self.boxes.len() == 1 && self.coordinates.is_none() {
            return collide_single_aabb(axis, &self.boxes[0], collision_box, max_dist);
        }
        self.collide_discrete_grid(axis, collision_box, max_dist)
    }
    fn collide_discrete_grid(&self, axis: Axis, moving: &AABB, mut distance: f64) -> f64 {
        let xs = self.coordinates(Axis::X);
        let ys = self.coordinates(Axis::Y);
        let zs = self.coordinates(Axis::Z);
        if xs.len() < 2 || ys.len() < 2 || zs.len() < 2 {
            return distance;
        }
        let (a_coords, b_coords, c_coords) = match axis {
            Axis::X => (&xs, &ys, &zs),
            Axis::Y => (&ys, &zs, &xs),
            Axis::Z => (&zs, &xs, &ys),
        };
        let coord = |coords: &[f64], index: isize| coords[index as usize];
        let find_index = |coords: &[f64], value: f64| -> isize {
            let mut low = 0usize;
            let mut high = coords.len();
            while low < high {
                let middle = low + (high - low) / 2;
                if value < coords[middle] {
                    high = middle;
                } else {
                    low = middle + 1;
                }
            }
            low as isize - 1
        };
        let max_a = axis_value(moving, axis, true);
        let min_a = axis_value(moving, axis, false);
        let a_min = find_index(a_coords, min_a + EPSILON);
        let a_max = find_index(a_coords, max_a - EPSILON);
        let b_min = find_index(
            b_coords,
            axis_value(moving, next_axis(axis), false) + EPSILON,
        )
        .max(0) as usize;
        let b_max = (find_index(
            b_coords,
            axis_value(moving, next_axis(axis), true) - EPSILON,
        ) + 1)
            .min((b_coords.len() - 1) as isize)
            .max(0) as usize;
        let c_min = find_index(
            c_coords,
            axis_value(moving, next_next_axis(axis), false) + EPSILON,
        )
        .max(0) as usize;
        let c_max = (find_index(
            c_coords,
            axis_value(moving, next_next_axis(axis), true) - EPSILON,
        ) + 1)
            .min((c_coords.len() - 1) as isize)
            .max(0) as usize;
        let is_full_wide = |a: usize, b: usize, c: usize| {
            let (x, y, z) = match axis {
                Axis::X => (a, b, c),
                Axis::Y => (c, a, b),
                Axis::Z => (b, c, a),
            };
            let point = Vec3::new(
                (xs[x] + xs[x + 1]) * 0.5,
                (ys[y] + ys[y + 1]) * 0.5,
                (zs[z] + zs[z + 1]) * 0.5,
            );
            self.boxes.iter().any(|bounds| bounds.contains(point))
        };
        let a_size = a_coords.len() - 1;
        if distance > 0.0 {
            let start = (a_max + 1).max(0) as usize;
            for a in start..a_size {
                for b in b_min..b_max {
                    for c in c_min..c_max {
                        if is_full_wide(a, b, c) {
                            let new_distance = coord(a_coords, a as isize) - max_a;
                            if new_distance >= -EPSILON {
                                distance = distance.min(new_distance);
                            }
                            return distance;
                        }
                    }
                }
            }
        } else if distance < 0.0 {
            let end = (a_min - 1).min(a_size as isize - 1);
            if end >= 0 {
                for a in (0..=end as usize).rev() {
                    for b in b_min..b_max {
                        for c in c_min..c_max {
                            if is_full_wide(a, b, c) {
                                let new_distance = coord(a_coords, a as isize + 1) - min_a;
                                if new_distance <= EPSILON {
                                    distance = distance.max(new_distance);
                                }
                                return distance;
                            }
                        }
                    }
                }
            }
        }
        distance
    }
}
fn next_axis(axis: Axis) -> Axis {
    match axis {
        Axis::X => Axis::Y,
        Axis::Y => Axis::Z,
        Axis::Z => Axis::X,
    }
}
fn next_next_axis(axis: Axis) -> Axis {
    next_axis(next_axis(axis))
}
fn axis_value(aabb: &AABB, axis: Axis, max: bool) -> f64 {
    match (axis, max) {
        (Axis::X, false) => aabb.min_x,
        (Axis::X, true) => aabb.max_x,
        (Axis::Y, false) => aabb.min_y,
        (Axis::Y, true) => aabb.max_y,
        (Axis::Z, false) => aabb.min_z,
        (Axis::Z, true) => aabb.max_z,
    }
}
fn collide_single_aabb(axis: Axis, shape: &AABB, moving: &AABB, mut distance: f64) -> f64 {
    let overlaps = match axis {
        Axis::X => {
            moving.max_y - EPSILON >= shape.min_y
                && moving.min_y + EPSILON < shape.max_y
                && moving.max_z - EPSILON >= shape.min_z
                && moving.min_z + EPSILON < shape.max_z
        }
        Axis::Y => {
            moving.max_x - EPSILON >= shape.min_x
                && moving.min_x + EPSILON < shape.max_x
                && moving.max_z - EPSILON >= shape.min_z
                && moving.min_z + EPSILON < shape.max_z
        }
        Axis::Z => {
            moving.max_x - EPSILON >= shape.min_x
                && moving.min_x + EPSILON < shape.max_x
                && moving.max_y - EPSILON >= shape.min_y
                && moving.min_y + EPSILON < shape.max_y
        }
    };
    if !overlaps {
        return distance;
    }
    let (shape_min, shape_max, moving_min, moving_max) = match axis {
        Axis::X => (shape.min_x, shape.max_x, moving.min_x, moving.max_x),
        Axis::Y => (shape.min_y, shape.max_y, moving.min_y, moving.max_y),
        Axis::Z => (shape.min_z, shape.max_z, moving.min_z, moving.max_z),
    };
    if distance > 0.0 && moving_max - EPSILON < shape_min {
        let new_distance = shape_min - moving_max;
        if new_distance >= -EPSILON {
            if new_distance < distance {
                distance = new_distance;
            }
        }
    } else if distance < 0.0 && moving_min + EPSILON >= shape_max {
        let new_distance = shape_max - moving_min;
        if new_distance <= EPSILON {
            if new_distance > distance {
                distance = new_distance;
            }
        }
    }
    distance
}
