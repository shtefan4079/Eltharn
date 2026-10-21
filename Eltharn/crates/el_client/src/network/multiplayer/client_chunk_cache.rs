#![allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}
impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        ChunkPos { x, z }
    }
    pub fn pack(self) -> i64 {
        ((self.z as i64) << 32) | (self.x as i64 & 0xFFFF_FFFF)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionChange {
    Added,
    Removed,
}
#[derive(Debug, Default)]
pub struct UpdateTrackingSets {
    pub added_empty_sections: Vec<i64>,
    pub removed_empty_sections: Vec<i64>,
    pub added_loaded_chunks: Vec<i64>,
    pub removed_loaded_chunks: Vec<i64>,
}
pub struct ChunkStorage {
    pub chunk_radius: i32,
    pub view_range: i32,
    chunks: Vec<Option<()>>,
    pub view_center_x: i32,
    pub view_center_z: i32,
    pub chunk_count: i32,
    updating_sets_index: usize,
    update_sets: [UpdateTrackingSets; 2],
}
#[allow(dead_code)]
impl ChunkStorage {
    pub fn new(chunk_radius: i32) -> Self {
        let view_range = chunk_radius * 2 + 1;
        let capacity = (view_range * view_range) as usize;
        ChunkStorage {
            chunk_radius,
            view_range,
            chunks: vec![None; capacity],
            view_center_x: 0,
            view_center_z: 0,
            chunk_count: 0,
            updating_sets_index: 0,
            update_sets: Default::default(),
        }
    }
    pub fn get_index(&self, chunk_x: i32, chunk_z: i32) -> usize {
        let z = chunk_z.rem_euclid(self.view_range) as usize;
        let x = chunk_x.rem_euclid(self.view_range) as usize;
        z * self.view_range as usize + x
    }
    pub fn in_range(&self, chunk_x: i32, chunk_z: i32) -> bool {
        (chunk_x - self.view_center_x).abs() <= self.chunk_radius
            && (chunk_z - self.view_center_z).abs() <= self.chunk_radius
    }
    pub fn get_chunk(&self, index: usize) -> Option<()> {
        self.chunks.get(index).and_then(|c| *c)
    }
    pub fn replace(&mut self, index: usize, new_chunk: Option<()>) -> Option<()> {
        let old = self.chunks[index].take();
        if old.is_some() {
            self.chunk_count -= 1;
        }
        self.chunks[index] = new_chunk;
        if new_chunk.is_some() {
            self.chunk_count += 1;
        }
        old
    }
    pub fn drop(&mut self, index: usize) -> bool {
        if self.chunks[index].is_some() {
            self.chunks[index] = None;
            self.chunk_count -= 1;
            true
        } else {
            false
        }
    }
    pub fn flip_update_tracking_sets(&mut self) {
        self.updating_sets_index = (self.updating_sets_index + 1) % 2;
        let sets = &mut self.update_sets[self.updating_sets_index];
        sets.added_empty_sections.clear();
        sets.removed_empty_sections.clear();
        sets.added_loaded_chunks.clear();
        sets.removed_loaded_chunks.clear();
    }
    pub fn current_sets(&self) -> &UpdateTrackingSets {
        &self.update_sets[self.updating_sets_index]
    }
    pub fn current_sets_mut(&mut self) -> &mut UpdateTrackingSets {
        &mut self.update_sets[self.updating_sets_index]
    }
}
pub struct ClientChunkCache {
    storage: ChunkStorage,
}
impl ClientChunkCache {
    pub fn new(server_chunk_radius: i32) -> Self {
        ClientChunkCache {
            storage: ChunkStorage::new(Self::calculate_storage_range(server_chunk_radius)),
        }
    }
    pub fn drop(&mut self, pos: ChunkPos) {
        if self.storage.in_range(pos.x, pos.z) {
            let index = self.storage.get_index(pos.x, pos.z);
            self.storage.drop(index);
        }
    }
    pub fn has_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        if !self.storage.in_range(chunk_x, chunk_z) {
            return false;
        }
        self.storage
            .get_chunk(self.storage.get_index(chunk_x, chunk_z))
            .is_some()
    }
    pub fn update_view_center(&mut self, x: i32, z: i32) {
        self.storage.view_center_x = x;
        self.storage.view_center_z = z;
    }
    pub fn update_view_radius(&mut self, view_range: i32) {
        let new_radius = Self::calculate_storage_range(view_range);
        if new_radius == self.storage.chunk_radius {
            return;
        }
        let mut new_storage = ChunkStorage::new(new_radius);
        new_storage.view_center_x = self.storage.view_center_x;
        new_storage.view_center_z = self.storage.view_center_z;
        self.storage = new_storage;
    }
    pub fn flip_update_tracking_sets(&mut self) {
        self.storage.flip_update_tracking_sets();
    }
    pub fn gather_stats(&self) -> String {
        format!(
            "{}, {}",
            self.storage.chunks.len(),
            self.get_loaded_chunks_count()
        )
    }
    pub fn get_loaded_chunks_count(&self) -> i32 {
        self.storage.chunk_count
    }
    pub fn current_sets(&self) -> &UpdateTrackingSets {
        self.storage.current_sets()
    }
    fn calculate_storage_range(view_range: i32) -> i32 {
        view_range.max(2) + 3
    }
}
