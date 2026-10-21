use super::VisibilitySet;
use el_math::Direction;
use std::collections::VecDeque;
use std::sync::OnceLock;
const SIZE: usize = 4096;
const EDGE_COUNT: usize = 1352;
pub struct VisGraph {
    opaque: [u64; SIZE / 64],
    empty: usize,
}
impl Default for VisGraph {
    fn default() -> Self {
        Self::new()
    }
}
impl VisGraph {
    pub fn new() -> Self {
        Self {
            opaque: [0; SIZE / 64],
            empty: SIZE,
        }
    }
    #[inline]
    const fn index(x: usize, y: usize, z: usize) -> usize {
        x | (z << 4) | (y << 8)
    }
    #[inline]
    fn get(bits: &[u64; SIZE / 64], index: usize) -> bool {
        bits[index >> 6] & (1u64 << (index & 63)) != 0
    }
    #[inline]
    fn set(bits: &mut [u64; SIZE / 64], index: usize) {
        bits[index >> 6] |= 1u64 << (index & 63);
    }
    pub fn set_opaque(&mut self, x: i32, y: i32, z: i32) {
        let index = Self::index((x & 15) as usize, (y & 15) as usize, (z & 15) as usize);
        if !Self::get(&self.opaque, index) {
            Self::set(&mut self.opaque, index);
            self.empty -= 1;
        }
    }
    pub fn resolve(&self) -> VisibilitySet {
        let mut visibility = VisibilitySet::default();
        if SIZE - self.empty < 256 {
            visibility.set_all(true);
            return visibility;
        }
        if self.empty == 0 {
            return visibility;
        }
        let mut visited = self.opaque;
        for &index in edge_indices() {
            if !Self::get(&visited, index) {
                let edges = Self::flood_fill(&mut visited, index);
                visibility.add(&edges);
            }
        }
        visibility
    }
    fn flood_fill(visited: &mut [u64; SIZE / 64], start: usize) -> Vec<Direction> {
        let mut edges = Vec::with_capacity(6);
        let mut queue = VecDeque::new();
        queue.push_back(start);
        Self::set(visited, start);
        while let Some(index) = queue.pop_front() {
            add_edges(index, &mut edges);
            for direction in Direction::VALUES {
                if let Some(neighbor) = neighbor_index(index, direction) {
                    if !Self::get(visited, neighbor) {
                        Self::set(visited, neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        edges
    }
}
fn edge_indices() -> &'static [usize; EDGE_COUNT] {
    static EDGES: OnceLock<[usize; EDGE_COUNT]> = OnceLock::new();
    EDGES.get_or_init(|| {
        let mut edges = [0; EDGE_COUNT];
        let mut n = 0;
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    if x == 0 || x == 15 || y == 0 || y == 15 || z == 0 || z == 15 {
                        edges[n] = VisGraph::index(x, y, z);
                        n += 1;
                    }
                }
            }
        }
        debug_assert_eq!(n, EDGE_COUNT);
        edges
    })
}
fn add_edges(index: usize, edges: &mut Vec<Direction>) {
    let x = index & 15;
    let z = (index >> 4) & 15;
    let y = (index >> 8) & 15;
    let candidates = [
        (x == 0, Direction::West),
        (x == 15, Direction::East),
        (y == 0, Direction::Down),
        (y == 15, Direction::Up),
        (z == 0, Direction::North),
        (z == 15, Direction::South),
    ];
    for (present, direction) in candidates {
        if present && !edges.contains(&direction) {
            edges.push(direction);
        }
    }
}
fn neighbor_index(index: usize, direction: Direction) -> Option<usize> {
    let x = index & 15;
    let z = (index >> 4) & 15;
    let y = (index >> 8) & 15;
    match direction {
        Direction::Down if y != 0 => Some(index - 256),
        Direction::Up if y != 15 => Some(index + 256),
        Direction::North if z != 0 => Some(index - 16),
        Direction::South if z != 15 => Some(index + 16),
        Direction::West if x != 0 => Some(index - 1),
        Direction::East if x != 15 => Some(index + 1),
        _ => None,
    }
}
