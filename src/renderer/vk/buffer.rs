use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::Result;
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator},
    MemoryLocation,
};

pub struct AllocatedBuffer {
    pub buffer: vk::Buffer,
    pub size: u64,

    allocation: Option<Allocation>,
    memory_allocator: Arc<Mutex<Allocator>>,
    device: Arc<ash::Device>,
}

impl AllocatedBuffer {
    pub fn new(
        buffer_size: u64,
        buffer_usage: vk::BufferUsageFlags,
        alloc_name: &str,
        alloc_loc: MemoryLocation,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let buffer = {
            let buffer_info = vk::BufferCreateInfo {
                size: buffer_size,
                usage: buffer_usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            unsafe { device.create_buffer(&buffer_info, None)? }
        };

        let reqs = unsafe { device.get_buffer_memory_requirements(buffer) };
        let allocation = memory_allocator
            .lock()?
            .allocate(&AllocationCreateDesc {
                name: alloc_name,
                requirements: reqs,
                location: alloc_loc,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
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
            size: buffer_size,

            allocation: Some(allocation),
            memory_allocator,
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

impl Drop for AllocatedBuffer {
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