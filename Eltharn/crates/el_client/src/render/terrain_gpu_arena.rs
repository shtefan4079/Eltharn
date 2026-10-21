use crate::render::baked_model::Vertex;
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::num::NonZeroU64;
pub const JAVA_TERRAIN_VERTEX_BANK_BYTES: u64 = 134_217_728;
pub const JAVA_TERRAIN_INDEX_BANK_BYTES: u64 = 33_554_432;
pub const JAVA_TERRAIN_INDEX_ALIGNMENT: u64 = 8;
#[cfg(not(target_arch = "wasm32"))]
const INDIRECT_SLOT_COUNT: usize = 3;
#[cfg(target_arch = "wasm32")]
const INDIRECT_SLOT_COUNT: usize = 2;
#[cfg(not(target_arch = "wasm32"))]
const STAGING_SLOT_COUNT: usize = 3;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainGpuBankId(u32);
impl TerrainGpuBankId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainGpuAllocation {
    bank: TerrainGpuBankId,
    vertex_offset: u64,
    vertex_capacity: u64,
    index_offset: u64,
    index_capacity: u64,
}
impl TerrainGpuAllocation {
    pub fn bank(self) -> TerrainGpuBankId {
        self.bank
    }
    pub fn index_offset_for_index(self, first_index: u32) -> Option<u64> {
        let relative = u64::from(first_index).checked_mul(std::mem::size_of::<u32>() as u64)?;
        (relative <= self.index_capacity)
            .then(|| self.index_offset.checked_add(relative))
            .flatten()
    }
    pub fn fits(self, vertex_bytes: u64, index_bytes: u64) -> bool {
        vertex_bytes <= self.vertex_capacity && index_bytes <= self.index_capacity
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainOpaqueDraw {
    pub bank: TerrainGpuBankId,
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
}
impl TerrainOpaqueDraw {
    pub fn indirect_args(self) -> wgpu::util::DrawIndexedIndirectArgs {
        wgpu::util::DrawIndexedIndirectArgs {
            index_count: self.index_count,
            instance_count: 1,
            first_index: self.first_index,
            base_vertex: self.base_vertex,
            first_instance: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainOpaqueBatch {
    pub bank: TerrainGpuBankId,
    pub draws: Vec<TerrainOpaqueDraw>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainOpaqueIndirectUpload {
    pub bank: TerrainGpuBankId,
    slot: usize,
    pub draw_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainGpuArenaError {
    EmptyMesh,
    AllocationTooLarge,
    OffsetOverflow,
    StagingWriteFailed,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainStagedUploadStats {
    pub sections: usize,
    pub bytes: u64,
    pub queue_writes: usize,
    pub copy_commands: usize,
}
#[derive(Clone, Debug)]
struct FreeRangeAllocator {
    capacity: u64,
    free: BTreeMap<u64, u64>,
}
impl FreeRangeAllocator {
    fn new(capacity: u64) -> Self {
        let mut free = BTreeMap::new();
        if capacity > 0 {
            free.insert(0, capacity);
        }
        Self { capacity, free }
    }
    fn allocate(&mut self, size: u64, alignment: u64) -> Option<(u64, u64)> {
        if size == 0 || alignment == 0 {
            return None;
        }
        let candidate = self.free.iter().find_map(|(&offset, &length)| {
            let aligned_offset = align_up(offset, alignment)?;
            let leading = aligned_offset.checked_sub(offset)?;
            let required = leading.checked_add(size)?;
            (required <= length).then_some((offset, length, aligned_offset, leading))
        })?;
        let (offset, length, aligned_offset, leading) = candidate;
        self.free.remove(&offset);
        if leading > 0 {
            self.free.insert(offset, leading);
        }
        let consumed = leading + size;
        let trailing = length - consumed;
        if trailing > 0 {
            self.free.insert(aligned_offset + size, trailing);
        }
        Some((aligned_offset, size))
    }
    fn free(&mut self, offset: u64, size: u64) {
        if size == 0 {
            return;
        }
        debug_assert!(offset <= self.capacity);
        debug_assert!(size <= self.capacity.saturating_sub(offset));
        let mut merged_offset = offset;
        let mut merged_size = size;
        if let Some((&previous_offset, &previous_size)) = self.free.range(..offset).next_back()
            && previous_offset.checked_add(previous_size) == Some(offset)
        {
            self.free.remove(&previous_offset);
            merged_offset = previous_offset;
            merged_size = merged_size.saturating_add(previous_size);
        }
        let end = merged_offset.saturating_add(merged_size);
        if let Some((&next_offset, &next_size)) = self.free.range(end..).next()
            && next_offset == end
        {
            self.free.remove(&next_offset);
            merged_size = merged_size.saturating_add(next_size);
        }
        self.free.insert(merged_offset, merged_size);
    }
    fn is_completely_free(&self) -> bool {
        self.free.len() == 1 && self.free.get(&0) == Some(&self.capacity)
    }
}
struct TerrainIndirectSlot {
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
}
impl Default for TerrainIndirectSlot {
    fn default() -> Self {
        Self {
            buffer: None,
            capacity: 0,
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
struct TerrainStagingSlot {
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
}
#[cfg(not(target_arch = "wasm32"))]
impl Default for TerrainStagingSlot {
    fn default() -> Self {
        Self {
            buffer: None,
            capacity: 0,
        }
    }
}
struct PendingTerrainUpload {
    allocation: TerrainGpuAllocation,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}
struct TerrainGpuBank {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertices: FreeRangeAllocator,
    indices: FreeRangeAllocator,
    indirect_slots: [TerrainIndirectSlot; INDIRECT_SLOT_COUNT],
}
impl TerrainGpuBank {
    fn new(
        device: &wgpu::Device,
        bank_index: usize,
        vertex_capacity: u64,
        index_capacity: u64,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Remote Terrain Vertex Bank {bank_index}")),
            size: vertex_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Remote Terrain Index Bank {bank_index}")),
            size: index_capacity,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            vertex_buffer,
            index_buffer,
            vertices: FreeRangeAllocator::new(vertex_capacity),
            indices: FreeRangeAllocator::new(index_capacity),
            indirect_slots: std::array::from_fn(|_| TerrainIndirectSlot::default()),
        }
    }
    fn allocate_pair(
        &mut self,
        vertex_bytes: u64,
        index_bytes: u64,
        vertex_alignment: u64,
    ) -> Option<((u64, u64), (u64, u64))> {
        let vertex = self.vertices.allocate(vertex_bytes, vertex_alignment)?;
        let Some(index) = self
            .indices
            .allocate(index_bytes, JAVA_TERRAIN_INDEX_ALIGNMENT)
        else {
            self.vertices.free(vertex.0, vertex.1);
            return None;
        };
        Some((vertex, index))
    }
}
pub struct TerrainGpuArena {
    vertex_stride: u64,
    vertex_alignment: u64,
    vertex_bank_capacity: u64,
    index_bank_capacity: u64,
    max_buffer_size: u64,
    banks: Vec<TerrainGpuBank>,
    indirect_slot: usize,
    #[cfg(not(target_arch = "wasm32"))]
    staging_slot: usize,
    #[cfg(not(target_arch = "wasm32"))]
    staging_slots: [TerrainStagingSlot; STAGING_SLOT_COUNT],
    pending_uploads: Vec<PendingTerrainUpload>,
}
impl TerrainGpuArena {
    pub fn new(vertex_stride: u64, max_buffer_size: u64) -> Self {
        assert!(vertex_stride > 0, "terrain vertex stride must be non-zero");
        let vertex_alignment = vertex_stride;
        let vertex_bank_capacity = JAVA_TERRAIN_VERTEX_BANK_BYTES.min(max_buffer_size);
        let index_bank_capacity = JAVA_TERRAIN_INDEX_BANK_BYTES.min(max_buffer_size);
        assert!(vertex_bank_capacity > 0 && index_bank_capacity > 0);
        Self {
            vertex_stride,
            vertex_alignment,
            vertex_bank_capacity,
            index_bank_capacity,
            max_buffer_size,
            banks: Vec::new(),
            indirect_slot: 0,
            #[cfg(not(target_arch = "wasm32"))]
            staging_slot: 0,
            #[cfg(not(target_arch = "wasm32"))]
            staging_slots: std::array::from_fn(|_| TerrainStagingSlot::default()),
            pending_uploads: Vec::new(),
        }
    }
    pub fn begin_frame(&mut self) {
        self.indirect_slot = (self.indirect_slot + 1) % INDIRECT_SLOT_COUNT;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.staging_slot = (self.staging_slot + 1) % STAGING_SLOT_COUNT;
        }
    }
    pub fn bank_count(&self) -> usize {
        self.banks.len()
    }
    pub fn stage_upload(
        &mut self,
        device: &wgpu::Device,
        previous: Option<TerrainGpuAllocation>,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
    ) -> Result<TerrainGpuAllocation, TerrainGpuArenaError> {
        if vertices.is_empty() || indices.is_empty() {
            return Err(TerrainGpuArenaError::EmptyMesh);
        }
        let (vertex_len, index_len) = terrain_mesh_byte_lengths(&vertices, &indices)?;
        if vertex_len > self.max_buffer_size || index_len > self.max_buffer_size {
            return Err(TerrainGpuArenaError::AllocationTooLarge);
        }
        let allocation = if let Some(previous) = previous
            && previous.fits(vertex_len, index_len)
            && self.banks.get(previous.bank.index()).is_some()
        {
            previous
        } else {
            let allocation = self.allocate_pair(device, vertex_len, index_len)?;
            if let Some(previous) = previous {
                self.free(previous);
            }
            allocation
        };
        self.pending_uploads.push(PendingTerrainUpload {
            allocation,
            vertices,
            indices,
        });
        Ok(allocation)
    }
    pub fn flush_staged_uploads(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<TerrainStagedUploadStats, TerrainGpuArenaError> {
        if self.pending_uploads.is_empty() {
            return Ok(TerrainStagedUploadStats::default());
        }
        let uploads = std::mem::take(&mut self.pending_uploads);
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (device, encoder);
            self.write_uploads_direct(queue, &uploads)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.flush_native_staged_uploads(device, queue, encoder, uploads)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn flush_native_staged_uploads(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        uploads: Vec<PendingTerrainUpload>,
    ) -> Result<TerrainStagedUploadStats, TerrainGpuArenaError> {
        let required = terrain_staging_bytes(&uploads)?;
        if required == 0 {
            return Err(TerrainGpuArenaError::EmptyMesh);
        }
        if required > self.max_buffer_size {
            return self.write_uploads_direct(queue, &uploads);
        }
        let current_capacity = self.staging_slots[self.staging_slot].capacity;
        let should_grow =
            self.staging_slots[self.staging_slot].buffer.is_none() || current_capacity < required;
        if should_grow {
            let capacity =
                round_up_to_copy_alignment(current_capacity.saturating_mul(2).max(required));
            if capacity > self.max_buffer_size {
                return self.write_uploads_direct(queue, &uploads);
            }
            let slot = &mut self.staging_slots[self.staging_slot];
            slot.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Remote Terrain Staging"),
                size: capacity,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            slot.capacity = capacity;
        }
        let staging = self.staging_slots[self.staging_slot]
            .buffer
            .as_ref()
            .expect("terrain staging buffer was allocated");
        let Some(mut writer) = queue.write_buffer_with(
            staging,
            0,
            NonZeroU64::new(required).expect("non-empty terrain staging write"),
        ) else {
            return self.write_uploads_direct(queue, &uploads);
        };
        let mut staging_offset = 0u64;
        for upload in &uploads {
            let vertex_bytes = bytemuck::cast_slice(&upload.vertices);
            let index_bytes = bytemuck::cast_slice(&upload.indices);
            copy_into_staging(&mut writer, staging_offset, vertex_bytes)?;
            staging_offset = next_staging_offset(staging_offset, vertex_bytes.len() as u64)?;
            copy_into_staging(&mut writer, staging_offset, index_bytes)?;
            staging_offset = next_staging_offset(staging_offset, index_bytes.len() as u64)?;
        }
        drop(writer);
        let mut stats = TerrainStagedUploadStats {
            sections: uploads.len(),
            bytes: 0,
            queue_writes: 1,
            copy_commands: 0,
        };
        let mut staging_offset = 0u64;
        for upload in uploads {
            let (vertex_bytes, index_bytes) =
                terrain_mesh_byte_lengths(&upload.vertices, &upload.indices)?;
            let Some(bank) = self.banks.get(upload.allocation.bank.index()) else {
                debug_assert!(false, "pending terrain upload references missing bank");
                continue;
            };
            encoder.copy_buffer_to_buffer(
                staging,
                staging_offset,
                &bank.vertex_buffer,
                upload.allocation.vertex_offset,
                vertex_bytes,
            );
            staging_offset = next_staging_offset(staging_offset, vertex_bytes)?;
            encoder.copy_buffer_to_buffer(
                staging,
                staging_offset,
                &bank.index_buffer,
                upload.allocation.index_offset,
                index_bytes,
            );
            staging_offset = next_staging_offset(staging_offset, index_bytes)?;
            stats.bytes = stats
                .bytes
                .checked_add(vertex_bytes)
                .and_then(|bytes| bytes.checked_add(index_bytes))
                .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
            stats.copy_commands += 2;
        }
        debug_assert_eq!(staging_offset, required);
        Ok(stats)
    }
    pub fn replace_staged_indices(
        &mut self,
        allocation: TerrainGpuAllocation,
        local_first_index: u32,
        indices: &[u32],
    ) -> bool {
        let Some(upload) = self
            .pending_uploads
            .iter_mut()
            .rev()
            .find(|upload| upload.allocation == allocation)
        else {
            return false;
        };
        let Ok(start) = usize::try_from(local_first_index) else {
            debug_assert!(false, "terrain staged index offset exceeds usize");
            return false;
        };
        let Some(end) = start.checked_add(indices.len()) else {
            debug_assert!(false, "terrain staged index range overflow");
            return false;
        };
        if end > upload.indices.len() {
            debug_assert!(false, "terrain staged index range exceeds allocation");
            return false;
        }
        upload.indices[start..end].copy_from_slice(indices);
        true
    }
    fn write_uploads_direct(
        &self,
        queue: &wgpu::Queue,
        uploads: &[PendingTerrainUpload],
    ) -> Result<TerrainStagedUploadStats, TerrainGpuArenaError> {
        let mut stats = TerrainStagedUploadStats {
            sections: uploads.len(),
            bytes: 0,
            queue_writes: 0,
            copy_commands: 0,
        };
        for upload in uploads {
            let (vertex_bytes, index_bytes) =
                terrain_mesh_byte_lengths(&upload.vertices, &upload.indices)?;
            let bank = self
                .banks
                .get(upload.allocation.bank.index())
                .ok_or(TerrainGpuArenaError::StagingWriteFailed)?;
            queue.write_buffer(
                &bank.vertex_buffer,
                upload.allocation.vertex_offset,
                bytemuck::cast_slice(&upload.vertices),
            );
            queue.write_buffer(
                &bank.index_buffer,
                upload.allocation.index_offset,
                bytemuck::cast_slice(&upload.indices),
            );
            stats.bytes = stats
                .bytes
                .checked_add(vertex_bytes)
                .and_then(|bytes| bytes.checked_add(index_bytes))
                .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
            stats.queue_writes += 2;
        }
        Ok(stats)
    }
    pub fn free(&mut self, allocation: TerrainGpuAllocation) {
        self.pending_uploads
            .retain(|upload| upload.allocation != allocation);
        let Some(bank) = self.banks.get_mut(allocation.bank.index()) else {
            debug_assert!(false, "terrain allocation references missing bank");
            return;
        };
        bank.vertices
            .free(allocation.vertex_offset, allocation.vertex_capacity);
        bank.indices
            .free(allocation.index_offset, allocation.index_capacity);
        while self.banks.last().is_some_and(|bank| {
            bank.vertices.is_completely_free() && bank.indices.is_completely_free()
        }) {
            self.banks.pop();
        }
    }
    pub fn clear(&mut self) {
        self.banks.clear();
        self.indirect_slot = 0;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.staging_slot = 0;
            self.staging_slots = std::array::from_fn(|_| TerrainStagingSlot::default());
        }
        self.pending_uploads.clear();
    }
    pub fn buffers(&self, bank: TerrainGpuBankId) -> Option<(&wgpu::Buffer, &wgpu::Buffer)> {
        let bank = self.banks.get(bank.index())?;
        Some((&bank.vertex_buffer, &bank.index_buffer))
    }
    pub fn opaque_draw(
        &self,
        allocation: TerrainGpuAllocation,
        index_count: u32,
    ) -> Result<TerrainOpaqueDraw, TerrainGpuArenaError> {
        self.draw_for_index_range(allocation, 0, index_count)
    }
    pub fn draw_for_index_range(
        &self,
        allocation: TerrainGpuAllocation,
        local_first_index: u32,
        index_count: u32,
    ) -> Result<TerrainOpaqueDraw, TerrainGpuArenaError> {
        let range_bytes = u64::from(local_first_index)
            .checked_add(u64::from(index_count))
            .and_then(|indices| indices.checked_mul(std::mem::size_of::<u32>() as u64))
            .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
        if range_bytes > allocation.index_capacity {
            return Err(TerrainGpuArenaError::OffsetOverflow);
        }
        let first_index = allocation
            .index_offset_for_index(local_first_index)
            .and_then(|offset| offset.checked_div(std::mem::size_of::<u32>() as u64))
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
        let base_vertex = allocation
            .vertex_offset
            .checked_div(self.vertex_stride)
            .and_then(|value| i32::try_from(value).ok())
            .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
        Ok(TerrainOpaqueDraw {
            bank: allocation.bank,
            first_index,
            index_count,
            base_vertex,
        })
    }
    pub fn group_opaque_draws(
        draws: impl IntoIterator<Item = TerrainOpaqueDraw>,
    ) -> Vec<TerrainOpaqueBatch> {
        let mut batches = Vec::<TerrainOpaqueBatch>::new();
        for draw in draws {
            if draw.index_count == 0 {
                continue;
            }
            if let Some(batch) = batches.iter_mut().find(|batch| batch.bank == draw.bank) {
                batch.draws.push(draw);
            } else {
                batches.push(TerrainOpaqueBatch {
                    bank: draw.bank,
                    draws: vec![draw],
                });
            }
        }
        batches
    }
    pub fn upload_opaque_indirect_batches(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batches: &[TerrainOpaqueBatch],
    ) -> Vec<TerrainOpaqueIndirectUpload> {
        let slot = self.indirect_slot;
        let mut uploads = Vec::with_capacity(batches.len());
        for batch in batches {
            let Some(bank) = self.banks.get_mut(batch.bank.index()) else {
                continue;
            };
            let draw_count =
                u32::try_from(batch.draws.len()).expect("terrain indirect draw count exceeds u32");
            if draw_count == 0 {
                continue;
            }
            let bytes = indirect_bytes(&batch.draws);
            let required = u64::try_from(bytes.len()).expect("indirect bytes exceed u64");
            let indirect = &mut bank.indirect_slots[slot];
            if indirect.buffer.is_none() || required > indirect.capacity {
                let capacity = round_up_to_copy_alignment(required);
                indirect.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Remote Terrain Opaque Indirect Draws"),
                    size: capacity,
                    usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                indirect.capacity = capacity;
            }
            queue.write_buffer(
                indirect
                    .buffer
                    .as_ref()
                    .expect("indirect terrain buffer was allocated"),
                0,
                &bytes,
            );
            uploads.push(TerrainOpaqueIndirectUpload {
                bank: batch.bank,
                slot,
                draw_count,
            });
        }
        uploads
    }
    pub fn indirect_buffer(&self, upload: TerrainOpaqueIndirectUpload) -> Option<&wgpu::Buffer> {
        self.banks
            .get(upload.bank.index())?
            .indirect_slots
            .get(upload.slot)?
            .buffer
            .as_ref()
    }
    fn allocate_pair(
        &mut self,
        device: &wgpu::Device,
        vertex_bytes: u64,
        index_bytes: u64,
    ) -> Result<TerrainGpuAllocation, TerrainGpuArenaError> {
        for (index, bank) in self.banks.iter_mut().enumerate() {
            if let Some((vertex, index_range)) =
                bank.allocate_pair(vertex_bytes, index_bytes, self.vertex_alignment)
            {
                return Ok(TerrainGpuAllocation {
                    bank: TerrainGpuBankId(
                        u32::try_from(index).map_err(|_| TerrainGpuArenaError::OffsetOverflow)?,
                    ),
                    vertex_offset: vertex.0,
                    vertex_capacity: vertex.1,
                    index_offset: index_range.0,
                    index_capacity: index_range.1,
                });
            }
        }
        let bank_index = self.banks.len();
        let vertex_capacity = self.vertex_bank_capacity.max(vertex_bytes);
        let index_capacity = self.index_bank_capacity.max(index_bytes);
        let mut bank = TerrainGpuBank::new(device, bank_index, vertex_capacity, index_capacity);
        let (vertex, index_range) = bank
            .allocate_pair(vertex_bytes, index_bytes, self.vertex_alignment)
            .ok_or(TerrainGpuArenaError::AllocationTooLarge)?;
        self.banks.push(bank);
        Ok(TerrainGpuAllocation {
            bank: TerrainGpuBankId(
                u32::try_from(bank_index).map_err(|_| TerrainGpuArenaError::OffsetOverflow)?,
            ),
            vertex_offset: vertex.0,
            vertex_capacity: vertex.1,
            index_offset: index_range.0,
            index_capacity: index_range.1,
        })
    }
}
fn align_up(value: u64, alignment: u64) -> Option<u64> {
    let remainder = value % alignment;
    value.checked_add((alignment - remainder) % alignment)
}
fn round_up_to_copy_alignment(required: u64) -> u64 {
    align_up(required, wgpu::COPY_BUFFER_ALIGNMENT).expect("indirect buffer size overflow")
}
fn terrain_mesh_byte_lengths(
    vertices: &[Vertex],
    indices: &[u32],
) -> Result<(u64, u64), TerrainGpuArenaError> {
    let vertex_count =
        u64::try_from(vertices.len()).map_err(|_| TerrainGpuArenaError::OffsetOverflow)?;
    let index_count =
        u64::try_from(indices.len()).map_err(|_| TerrainGpuArenaError::OffsetOverflow)?;
    let vertex_bytes = vertex_count
        .checked_mul(std::mem::size_of::<Vertex>() as u64)
        .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
    let index_bytes = index_count
        .checked_mul(std::mem::size_of::<u32>() as u64)
        .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
    Ok((vertex_bytes, index_bytes))
}
#[cfg(not(target_arch = "wasm32"))]
fn next_staging_offset(offset: u64, byte_count: u64) -> Result<u64, TerrainGpuArenaError> {
    let end = offset
        .checked_add(byte_count)
        .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
    align_up(end, wgpu::COPY_BUFFER_ALIGNMENT).ok_or(TerrainGpuArenaError::OffsetOverflow)
}
#[cfg(not(target_arch = "wasm32"))]
fn terrain_staging_bytes(uploads: &[PendingTerrainUpload]) -> Result<u64, TerrainGpuArenaError> {
    uploads.iter().try_fold(0u64, |offset, upload| {
        let (vertex_bytes, index_bytes) =
            terrain_mesh_byte_lengths(&upload.vertices, &upload.indices)?;
        let offset = next_staging_offset(offset, vertex_bytes)?;
        next_staging_offset(offset, index_bytes)
    })
}
#[cfg(not(target_arch = "wasm32"))]
fn copy_into_staging(
    writer: &mut wgpu::QueueWriteBufferView<'_>,
    offset: u64,
    source: &[u8],
) -> Result<(), TerrainGpuArenaError> {
    let start = usize::try_from(offset).map_err(|_| TerrainGpuArenaError::OffsetOverflow)?;
    let end = start
        .checked_add(source.len())
        .ok_or(TerrainGpuArenaError::OffsetOverflow)?;
    writer.as_mut()[start..end].copy_from_slice(source);
    Ok(())
}
fn indirect_bytes(draws: &[TerrainOpaqueDraw]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        draws.len() * std::mem::size_of::<wgpu::util::DrawIndexedIndirectArgs>(),
    );
    for draw in draws {
        bytes.extend_from_slice(draw.indirect_args().as_bytes());
    }
    bytes
}
