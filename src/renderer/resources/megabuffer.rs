use crate::renderer::resources::buffer::Buffer;
use crate::renderer::contexts::device_ctx::transfer_ctx::TransferContext;
use ash::vk;
use color_eyre::eyre::{eyre, OptionExt};
use color_eyre::Result;
use gpu_allocator::vulkan::Allocator;
use gpu_allocator::MemoryLocation;
use std::sync::{Arc, Mutex};

#[repr(transparent)]
pub struct Megabuffer(Arc<Mutex<MegabufferInner>>);

pub struct FreeMegabufferRegion {
    offset: u64,
    size: u64,
}

pub struct AllocatedMegabufferRegion {
    offset: u64,
    size: u64,
    megabuffer: Megabuffer,
}

impl AllocatedMegabufferRegion {
    pub fn write<T>(&mut self, data: &[T]) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        self.megabuffer.write(data, self)
    }
}

struct MegabufferInner {
    buffer: Buffer,
    staging_buffer: Buffer,
    free_regions: Vec<FreeMegabufferRegion>,
    mem_loc: MemoryLocation,
    alignment: u64,

    transfer_context: Arc<TransferContext>,
}

pub trait MegabufferExt {
    fn new(
        size: u64,
        usage: vk::BufferUsageFlags,
        alignment: u64,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: Arc<TransferContext>,
    ) -> Result<Self>;
    fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion>;
    fn deallocate_region(&self, region: AllocatedMegabufferRegion) -> Result<()>;
    fn defragment(&self) -> Result<()>;
    fn upload(&self) -> Result<()>;
    fn write<T>(
        &self,
        data: &[T],
        region: &AllocatedMegabufferRegion,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy;
}

impl MegabufferExt for Megabuffer {
    fn new(
        size: u64,
        usage: vk::BufferUsageFlags,
        alignment: u64,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: Arc<TransferContext>,
    ) -> Result<Megabuffer> {
        let mem_loc = MemoryLocation::GpuOnly;
        let buffer = Buffer::new(
            size,
            usage,
            "Buffer Allocator Buffer Allocation",
            mem_loc,
            memory_allocator.clone(),
            device.clone(),
        )?;

        let staging_buffer = Buffer::new(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            "Buffer Allocator Staging Buffer Allocation",
            MemoryLocation::CpuToGpu,
            memory_allocator,
            device,
        )?;

        Ok(Megabuffer(Arc::new(Mutex::new(MegabufferInner {
            buffer,
            staging_buffer,
            free_regions: vec![FreeMegabufferRegion {
                offset: 0,
                size,
            }],
            mem_loc,
            alignment,
            transfer_context,
        }))))
    }

    fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let mut guard = self
            .lock()
            .map_err(|e| eyre!(e.to_string()))?;

        let aligned_size = guard.aligned_size(size);
        let free_region_index = guard
            .find_free_region_for_allocation(aligned_size)
            .ok_or_eyre("Failed to find free region for allocation")?;

        // Remove the free region from the free regions vector
        let free_region = guard.free_regions.remove(free_region_index);
        let allocated_region = AllocatedMegabufferRegion {
            offset: free_region.offset,
            size: free_region.size,
            megabuffer: self.clone(),
        };

        Ok(allocated_region)
    }

    fn deallocate_region(&self, region: AllocatedMegabufferRegion) -> Result<()> {
        let mut guard = self
            .lock()
            .map_err(|e| eyre!(e.to_string()))?;

        let mut left_index = None; // Some if there is a free region to the left of the deallocated region
        let mut right_index = None; // Some if there is a free region to the right of the deallocated region

        for (i, free_region) in guard.free_regions.iter().enumerate() {
            if free_region.offset + free_region.size == region.offset {
                left_index = Some(i);
            } else if region.offset + region.size == free_region.offset {
                right_index = Some(i);
            }
        }

        match (left_index, right_index) {
            (Some(left), Some(right)) => {
                guard.free_regions[left].size += region.size + guard.free_regions[right].size;
                guard.free_regions.remove(right);
            }
            (Some(left), None) => {
                guard.free_regions[left].size += region.size;
            }
            (None, Some(right)) => {
                guard.free_regions[right].offset = region.offset;
                guard.free_regions[right].size += region.size;
            }
            (None, None) => {
                let region = FreeMegabufferRegion {
                    offset: region.offset,
                    size: region.size,
                };
                guard.free_regions.push(region);
                guard.free_regions.sort_by_key(|r| r.offset);
            }
        }

        Ok(())
    }

    fn defragment(&self) -> Result<()> {
        let mut guard = self
            .lock()
            .map_err(|e| eyre!(e.to_string()))?;

        guard.free_regions.sort_by_key(|r| r.offset);

        // Merge adjacent free regions
        let mut i = 0;
        while i < guard.free_regions.len() - 1 {
            if guard.free_regions[i].offset + guard.free_regions[i].size == guard.free_regions[i + 1].offset {
                guard.free_regions[i].size += guard.free_regions[i + 1].size;
                guard.free_regions.remove(i + 1);
            } else {
                i += 1;
            }
        }

        Ok(())
    }

    fn upload(&self) -> Result<()> {
        let guard = self
            .lock()
            .map_err(|e| eyre!(e.to_string()))?;

        guard.transfer_context.immediate_submit(
            |cmd: vk::CommandBuffer, device: &ash::Device| {
                let copy_regions = guard.free_regions
                    .iter()
                    .map(|region| {
                        vk::BufferCopy {
                            src_offset: region.offset,
                            dst_offset: region.offset,
                            size: region.size,
                        }
                    })
                    .collect::<Vec<vk::BufferCopy>>();

                unsafe {
                    device.cmd_copy_buffer(
                        cmd,
                        guard.staging_buffer.buffer,
                        guard.buffer.buffer,
                        &copy_regions,
                    );
                }

                Ok(())
            },
        )?;

        Ok(())
    }

    fn write<T>(
        &self,
        data: &[T],
        region: &AllocatedMegabufferRegion,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        if (data.len() * size_of::<T>()) as u64 > region.size {
            return Err(eyre!("Data too large for region"));
        }

        let mut guard = self
            .lock()
            .map_err(|e| eyre!(e.to_string()))?;

        guard.staging_buffer.write(data, region.offset as usize)
    }

}

impl MegabufferInner {
    fn aligned_size(&self, size: u64) -> u64 {
        (size + self.alignment - 1) & !(self.alignment - 1)
    }

    /// Find a free region that can fit the allocation and splits it into 2 free regions if possible
    /// Returns the index of the free region that fits the allocation
    fn find_free_region_for_allocation(
        &mut self,
        alloc_size: u64
    ) -> Option<usize> {
        let (
            region_index,
            new_region,
        ) = self.free_regions.iter_mut()
            .enumerate()
            // Find the first free region that can fit the allocation
            .find(|(_, region)| region.size >= alloc_size)
            .map(|(i, region)| {
                // Split the free region into 2 regions:
                // 1. A free region that fits the allocation exactly
                // 2. The remaining free region
                let offset = region.offset;
                region.offset += alloc_size;
                region.size -= alloc_size;
                (
                    // Index of the remaining free region
                    i,

                    // The free region that fits the allocation exactly,
                    // ready to be inserted into the free regions vector
                    FreeMegabufferRegion {
                        offset,
                        size: alloc_size,
                    },
                )
            })?;

        // Insert the new free region into the free regions vector
        if self.free_regions[region_index].size == 0 {
            self.free_regions[region_index] = new_region;
        } else {
            self.free_regions.insert(region_index, new_region);
        }

        Some(region_index)
    }
}
