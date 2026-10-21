use bytemuck::Pod;
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use std::num::NonZeroU64;
struct Slot {
    buffer: Option<wgpu::Buffer>,
    bind_groups: Vec<wgpu::BindGroup>,
    staging: Vec<u8>,
    capacity: usize,
    used: usize,
}
impl Slot {
    fn new() -> Self {
        Self {
            buffer: None,
            bind_groups: Vec::new(),
            staging: Vec::new(),
            capacity: 0,
            used: 0,
        }
    }
    fn reset(&mut self) {
        self.staging.clear();
        self.used = 0;
    }
}
pub struct FrameUniformArena<T: Pod + Copy> {
    label: String,
    stride: usize,
    slots: Vec<Slot>,
    active_slot: usize,
    _marker: PhantomData<T>,
}
impl<T: Pod + Copy> FrameUniformArena<T> {
    pub fn new(device: &wgpu::Device, label: impl Into<String>) -> Self {
        let alignment = device
            .limits()
            .min_uniform_buffer_offset_alignment
            .max(align_of::<T>() as u32) as usize;
        let stride = align_up(size_of::<T>(), alignment.max(1));
        let slot_count = if cfg!(target_arch = "wasm32") { 2 } else { 3 };
        Self {
            label: label.into(),
            stride,
            slots: (0..slot_count).map(|_| Slot::new()).collect(),
            active_slot: 0,
            _marker: PhantomData,
        }
    }
    pub fn begin_frame(&mut self) {
        self.active_slot = (self.active_slot + 1) % self.slots.len();
        self.slots[self.active_slot].reset();
    }
    pub fn push(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        value: T,
    ) -> usize {
        let slot = &mut self.slots[self.active_slot];
        let index = slot.used;
        let required = index.checked_add(1).expect("uniform arena index overflow");
        if required > slot.capacity {
            self.grow_slot(device, layout, required);
        }
        let slot = &mut self.slots[self.active_slot];
        let offset = index * self.stride;
        let bytes = bytemuck::bytes_of(&value);
        let end = offset + size_of::<T>();
        if slot.staging.len() < end {
            slot.staging.resize(end, 0);
        }
        slot.staging[offset..end].copy_from_slice(bytes);
        slot.used = required;
        index
    }
    pub fn upload(&self, queue: &wgpu::Queue) {
        let slot = &self.slots[self.active_slot];
        if slot.used == 0 {
            return;
        }
        queue.write_buffer(
            slot.buffer
                .as_ref()
                .expect("uniform arena buffer missing after push"),
            0,
            &slot.staging,
        );
    }
    pub fn bind_group(&self, index: usize) -> &wgpu::BindGroup {
        self.slots[self.active_slot]
            .bind_groups
            .get(index)
            .expect("uniform arena bind-group index out of range")
    }
    fn grow_slot(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        required: usize,
    ) {
        let slot = &mut self.slots[self.active_slot];
        let capacity = required.next_power_of_two().max(2);
        let byte_size = capacity
            .checked_mul(self.stride)
            .and_then(|size| u64::try_from(size).ok())
            .expect("uniform arena byte size overflow");
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Buffer", self.label)),
            size: byte_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binding_size =
            NonZeroU64::new(size_of::<T>() as u64).expect("uniform type must have non-zero size");
        let bind_groups = (0..capacity)
            .map(|index| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: (index * self.stride) as u64,
                            size: Some(binding_size),
                        }),
                    }],
                    label: Some(&format!("{} Bind Group", self.label)),
                })
            })
            .collect();
        slot.buffer = Some(buffer);
        slot.bind_groups = bind_groups;
        slot.capacity = capacity;
    }
}
#[inline]
fn align_up(value: usize, alignment: usize) -> usize {
    debug_assert!(alignment > 0);
    value
        .checked_add(alignment - 1)
        .expect("uniform alignment overflow")
        / alignment
        * alignment
}
