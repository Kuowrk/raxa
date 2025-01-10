use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator},
    MemoryLocation,
};

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub size: u64,

    allocation: Option<Allocation>,
    memory_allocator: Arc<Mutex<Allocator>>,
    device: Arc<ash::Device>,
}

impl Buffer {
    pub fn new(
        size: u64,
        usage: vk::BufferUsageFlags,
        name: &str,
        mem_loc: MemoryLocation,
        mem_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let buffer = {
            let buffer_info = vk::BufferCreateInfo {
                size,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            unsafe { device.create_buffer(&buffer_info, None)? }
        };

        let requirements = unsafe {
            device.get_buffer_memory_requirements(buffer)
        };
        let allocation = mem_allocator
            .lock()
            .map_err(|e| eyre!(e.to_string()))?
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location: mem_loc,
                linear: true,
                allocation_scheme: AllocationScheme::DedicatedBuffer(buffer)
            })?;

        unsafe {
            device.bind_buffer_memory(
                buffer,
                allocation.memory(),
                allocation.offset(),
            )?;
        }

        Ok(Self {
            buffer,
            size,

            allocation: Some(allocation),
            memory_allocator: mem_allocator,
            device,
        })
    }

    pub fn write<T>(
        &mut self,
        data: &[T],
        start_offset: usize,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        Ok(presser::copy_from_slice_to_offset(
            data,
            self.allocation.as_mut().unwrap(),
            start_offset,
        )?)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.memory_allocator
                .lock()
                .unwrap()
                .free(self.allocation.take().unwrap())
                .unwrap();
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}