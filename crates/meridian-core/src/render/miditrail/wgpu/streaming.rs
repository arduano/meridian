use std::marker::PhantomData;

use bytemuck::{Pod, cast_slice};

const MIN_STREAMING_VERTICES: usize = 1024;
const MAX_STREAMING_BYTES: usize = 64 * 1024 * 1024;

pub struct StreamingVertexBuffer<T> {
    buffer: wgpu::Buffer,
    capacity: usize,
    label: &'static str,
    _marker: PhantomData<T>,
}

impl<T: Pod> StreamingVertexBuffer<T> {
    pub fn new(device: &wgpu::Device, label: &'static str) -> Self {
        Self {
            buffer: create_buffer::<T>(device, label, MIN_STREAMING_VERTICES),
            capacity: MIN_STREAMING_VERTICES,
            label,
            _marker: PhantomData,
        }
    }

    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[T]) {
        if vertices.is_empty() {
            return;
        }
        debug_assert!(vertices.len() <= Self::max_chunk_len());
        self.ensure_capacity(device, vertices.len());
        queue.write_buffer(&self.buffer, 0, cast_slice(vertices));
    }

    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }

    fn ensure_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.capacity {
            return;
        }
        let new_capacity = required
            .next_power_of_two()
            .max(MIN_STREAMING_VERTICES)
            .min(Self::max_chunk_len());
        self.buffer = create_buffer::<T>(device, self.label, new_capacity);
        self.capacity = new_capacity;
    }

    pub fn max_chunk_len() -> usize {
        (MAX_STREAMING_BYTES / std::mem::size_of::<T>()).max(1)
    }
}

pub struct StreamingBufferPool<T> {
    buffers: Vec<StreamingVertexBuffer<T>>,
    label: &'static str,
    _marker: PhantomData<T>,
}

impl<T: Pod> StreamingBufferPool<T> {
    pub fn new(label: &'static str) -> Self {
        Self {
            buffers: Vec::new(),
            label,
            _marker: PhantomData,
        }
    }

    pub fn write_chunk(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk_index: usize,
        vertices: &[T],
    ) {
        if self.buffers.len() <= chunk_index {
            self.buffers
                .resize_with(chunk_index + 1, || StreamingVertexBuffer::new(device, self.label));
        }
        self.buffers[chunk_index].write(device, queue, vertices);
    }

    pub fn slice(&self, chunk_index: usize) -> wgpu::BufferSlice<'_> {
        self.buffers[chunk_index].slice()
    }

    pub fn max_chunk_len() -> usize {
        StreamingVertexBuffer::<T>::max_chunk_len()
    }
}

fn create_buffer<T: Pod>(device: &wgpu::Device, label: &'static str, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (capacity * std::mem::size_of::<T>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
