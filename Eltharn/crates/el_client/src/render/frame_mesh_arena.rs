use bytemuck::Pod;
use std::mem;
use std::ops::Range;
pub const FRAME_MESH_CAPACITY_INCREMENT: u64 = 262_144;
#[cfg(not(target_arch = "wasm32"))]
const FRAME_MESH_BUFFER_SLOT_COUNT: usize = 2;
#[cfg(target_arch = "wasm32")]
const FRAME_MESH_BUFFER_SLOT_COUNT: usize = 1;
const GPU_COPY_ALIGNMENT: usize = wgpu::COPY_BUFFER_ALIGNMENT as usize;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameMeshArenaStats {
    append_count: u32,
    staged_vertex_bytes: usize,
    staged_index_bytes: usize,
    queue_write_count: u8,
}
impl FrameMeshArenaStats {
    pub fn append_count(self) -> u32 {
        self.append_count
    }
    pub fn staged_vertex_bytes(self) -> usize {
        self.staged_vertex_bytes
    }
    pub fn staged_index_bytes(self) -> usize {
        self.staged_index_bytes
    }
    pub fn queue_write_count(self) -> u8 {
        self.queue_write_count
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameMeshSlice {
    generation: u64,
    vertex_offset: u64,
    vertex_size: u64,
    index_offset: u64,
    index_size: u64,
    index_count: u32,
}
impl FrameMeshSlice {
    pub fn vertex_range(self) -> Range<u64> {
        self.vertex_offset..self.vertex_offset + self.vertex_size
    }
    pub fn index_range(self) -> Range<u64> {
        self.index_offset..self.index_offset + self.index_size
    }
}
pub struct FrameMeshBinding<'a> {
    vertex_buffer: &'a wgpu::Buffer,
    index_buffer: &'a wgpu::Buffer,
    vertex_range: Range<u64>,
    index_range: Range<u64>,
    index_count: u32,
}
impl<'a> FrameMeshBinding<'a> {
    pub fn vertex_buffer(&self) -> &'a wgpu::Buffer {
        self.vertex_buffer
    }
    pub fn index_buffer(&self) -> &'a wgpu::Buffer {
        self.index_buffer
    }
    pub fn vertex_range(&self) -> Range<u64> {
        self.vertex_range.clone()
    }
    pub fn index_range(&self) -> Range<u64> {
        self.index_range.clone()
    }
    pub fn index_count(&self) -> u32 {
        self.index_count
    }
    pub fn draw_indexed_args<V: Pod>(&self) -> (u32, u32, i32) {
        whole_buffer_draw_indexed_args::<V>(&self.vertex_range, &self.index_range, self.index_count)
    }
}
fn whole_buffer_draw_indexed_args<V: Pod>(
    vertex_range: &Range<u64>,
    index_range: &Range<u64>,
    expected_index_count: u32,
) -> (u32, u32, i32) {
    let vertex_stride = u64::try_from(mem::size_of::<V>()).expect("vertex stride exceeds u64");
    let index_stride = u64::try_from(mem::size_of::<u32>()).expect("index stride exceeds u64");
    assert!(vertex_stride > 0, "zero-sized vertex types cannot be drawn");
    debug_assert_eq!(vertex_range.start % vertex_stride, 0);
    debug_assert_eq!(index_range.start % index_stride, 0);
    debug_assert_eq!(index_range.end % index_stride, 0);
    let Ok(first_index) = u32::try_from(index_range.start / index_stride) else {
        debug_assert!(false, "frame mesh first index exceeds u32; skipping draw");
        return (0, 0, 0);
    };
    let Ok(index_count) = u32::try_from((index_range.end - index_range.start) / index_stride)
    else {
        debug_assert!(false, "frame mesh index count exceeds u32; skipping draw");
        return (0, 0, 0);
    };
    let Ok(base_vertex) = i32::try_from(vertex_range.start / vertex_stride) else {
        debug_assert!(false, "frame mesh base vertex exceeds i32; skipping draw");
        return (0, 0, 0);
    };
    debug_assert_eq!(index_count, expected_index_count);
    (first_index, index_count, base_vertex)
}
#[derive(Clone, Copy)]
pub struct UploadedFrameMeshArena<'a> {
    vertex_buffer: &'a wgpu::Buffer,
    index_buffer: &'a wgpu::Buffer,
    generation: u64,
}
impl<'a> UploadedFrameMeshArena<'a> {
    pub fn resolve(self, mesh: FrameMeshSlice) -> FrameMeshBinding<'a> {
        assert_current_generation(mesh, self.generation);
        FrameMeshBinding {
            vertex_buffer: self.vertex_buffer,
            index_buffer: self.index_buffer,
            vertex_range: mesh.vertex_range(),
            index_range: mesh.index_range(),
            index_count: mesh.index_count,
        }
    }
}
#[derive(Default)]
struct FrameMeshBufferSlot {
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    index_buffer: Option<wgpu::Buffer>,
    index_capacity: u64,
}
impl FrameMeshBufferSlot {
    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        vertex_bytes: &[u8],
        index_bytes: &[u8],
    ) {
        debug_assert!(!vertex_bytes.is_empty());
        debug_assert!(!index_bytes.is_empty());
        let required_vertex_capacity = vertex_bytes.len() as u64;
        if self.vertex_buffer.is_none() || required_vertex_capacity > self.vertex_capacity {
            let capacity = rounded_buffer_capacity(required_vertex_capacity);
            self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{label} Vertex Buffer")),
                size: capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertex_capacity = capacity;
        }
        queue.write_buffer(
            self.vertex_buffer
                .as_ref()
                .expect("frame mesh vertex buffer was just allocated"),
            0,
            vertex_bytes,
        );
        let required_index_capacity = index_bytes.len() as u64;
        if self.index_buffer.is_none() || required_index_capacity > self.index_capacity {
            let capacity = rounded_buffer_capacity(required_index_capacity);
            self.index_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{label} Index Buffer")),
                size: capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.index_capacity = capacity;
        }
        queue.write_buffer(
            self.index_buffer
                .as_ref()
                .expect("frame mesh index buffer was just allocated"),
            0,
            index_bytes,
        );
    }
}
pub struct FrameMeshArena {
    label: String,
    vertex_staging: Vec<u8>,
    index_staging: Vec<u8>,
    slots: [FrameMeshBufferSlot; FRAME_MESH_BUFFER_SLOT_COUNT],
    next_slot: usize,
    active_slot: Option<usize>,
    generation: u64,
    frame_begun: bool,
    uploaded: bool,
    append_count: u32,
    queue_write_count: u8,
}
impl FrameMeshArena {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            vertex_staging: Vec::new(),
            index_staging: Vec::new(),
            slots: std::array::from_fn(|_| FrameMeshBufferSlot::default()),
            next_slot: 0,
            active_slot: None,
            generation: 0,
            frame_begun: false,
            uploaded: false,
            append_count: 0,
            queue_write_count: 0,
        }
    }
    pub fn begin_frame(&mut self) {
        self.vertex_staging.clear();
        self.index_staging.clear();
        self.active_slot = None;
        self.generation = self
            .generation
            .checked_add(1)
            .expect("FrameMeshArena generation overflow");
        self.frame_begun = true;
        self.uploaded = false;
        self.append_count = 0;
        self.queue_write_count = 0;
    }
    pub fn append_indexed<V: Pod>(
        &mut self,
        vertices: &[V],
        indices: &[u32],
    ) -> Option<FrameMeshSlice> {
        self.append_indexed_layers(vertices, [indices])[0]
    }
    pub fn append_indexed_layers<V: Pod, const N: usize>(
        &mut self,
        vertices: &[V],
        index_layers: [&[u32]; N],
    ) -> [Option<FrameMeshSlice>; N] {
        assert!(self.frame_begun, "begin_frame must precede append_indexed");
        assert!(!self.uploaded, "cannot append a frame mesh after upload");
        let mut slices = [None; N];
        if vertices.is_empty() || index_layers.iter().all(|indices| indices.is_empty()) {
            return slices;
        }
        let vertex_stride = mem::size_of::<V>();
        assert!(
            vertex_stride > 0,
            "zero-sized vertex types cannot be uploaded"
        );
        let vertex_alignment = least_common_multiple(vertex_stride, GPU_COPY_ALIGNMENT);
        let vertex_offset = align_up(self.vertex_staging.len(), vertex_alignment);
        self.vertex_staging.resize(vertex_offset, 0);
        let vertex_bytes = bytemuck::cast_slice(vertices);
        self.vertex_staging.extend_from_slice(vertex_bytes);
        let padded_vertex_end = align_up(self.vertex_staging.len(), GPU_COPY_ALIGNMENT);
        self.vertex_staging.resize(padded_vertex_end, 0);
        for (slice, indices) in slices.iter_mut().zip(index_layers) {
            if indices.is_empty() {
                continue;
            }
            let index_offset = align_up(self.index_staging.len(), mem::size_of::<u32>());
            self.index_staging.resize(index_offset, 0);
            let index_bytes = bytemuck::cast_slice(indices);
            self.index_staging.extend_from_slice(index_bytes);
            self.append_count = self
                .append_count
                .checked_add(1)
                .expect("FrameMeshArena append counter overflow");
            *slice = Some(FrameMeshSlice {
                generation: self.generation,
                vertex_offset: u64::try_from(vertex_offset)
                    .expect("frame mesh vertex offset exceeds u64"),
                vertex_size: u64::try_from(vertex_bytes.len())
                    .expect("frame mesh vertex size exceeds u64"),
                index_offset: u64::try_from(index_offset)
                    .expect("frame mesh index offset exceeds u64"),
                index_size: u64::try_from(index_bytes.len())
                    .expect("frame mesh index size exceeds u64"),
                index_count: u32::try_from(indices.len())
                    .expect("frame mesh has more than u32::MAX indices"),
            });
        }
        slices
    }
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        assert!(self.frame_begun, "begin_frame must precede upload");
        assert!(
            !self.uploaded,
            "FrameMeshArena can only upload once per frame"
        );
        self.uploaded = true;
        if self.vertex_staging.is_empty() || self.index_staging.is_empty() {
            debug_assert!(self.vertex_staging.is_empty() && self.index_staging.is_empty());
            self.active_slot = None;
            return;
        }
        let slot_index = self.select_next_slot();
        self.slots[slot_index].upload(
            device,
            queue,
            &self.label,
            &self.vertex_staging,
            &self.index_staging,
        );
        self.active_slot = Some(slot_index);
        self.queue_write_count = 2;
    }
    pub fn uploaded(&self) -> Option<UploadedFrameMeshArena<'_>> {
        if !self.uploaded {
            return None;
        }
        let slot = self.active_slot.and_then(|index| self.slots.get(index))?;
        Some(UploadedFrameMeshArena {
            vertex_buffer: slot.vertex_buffer.as_ref()?,
            index_buffer: slot.index_buffer.as_ref()?,
            generation: self.generation,
        })
    }
    pub fn stats(&self) -> FrameMeshArenaStats {
        FrameMeshArenaStats {
            append_count: self.append_count,
            staged_vertex_bytes: self.vertex_staging.len(),
            staged_index_bytes: self.index_staging.len(),
            queue_write_count: self.queue_write_count,
        }
    }
    fn select_next_slot(&mut self) -> usize {
        let selected = self.next_slot;
        self.next_slot = (selected + 1) % FRAME_MESH_BUFFER_SLOT_COUNT;
        selected
    }
}
impl Default for FrameMeshArena {
    fn default() -> Self {
        Self::new("Frame Mesh Arena")
    }
}
fn rounded_buffer_capacity(required: u64) -> u64 {
    if required == 0 {
        return 0;
    }
    let full_increments = required / FRAME_MESH_CAPACITY_INCREMENT;
    let increments = full_increments + u64::from(required % FRAME_MESH_CAPACITY_INCREMENT != 0);
    increments
        .checked_mul(FRAME_MESH_CAPACITY_INCREMENT)
        .expect("frame mesh buffer capacity overflow")
}
fn assert_current_generation(mesh: FrameMeshSlice, generation: u64) {
    assert_eq!(
        mesh.generation, generation,
        "attempted to resolve a stale FrameMeshSlice"
    );
}
fn align_up(value: usize, alignment: usize) -> usize {
    debug_assert!(alignment > 0);
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value
            .checked_add(alignment - remainder)
            .expect("frame mesh staging offset overflow")
    }
}
fn least_common_multiple(a: usize, b: usize) -> usize {
    a.checked_div(greatest_common_divisor(a, b))
        .and_then(|value| value.checked_mul(b))
        .expect("frame mesh vertex alignment overflow")
}
fn greatest_common_divisor(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}
