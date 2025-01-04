use std::sync::{Arc, Mutex, PoisonError};
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{eyre, OptionExt};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::Allocator;
use crate::renderer::internals::buffer::Buffer;
use crate::renderer::internals::transfer_context::TransferContext;

pub struct FreeMegabufferRegion {
    offset: u64,
    size: u64,
}

pub struct AllocatedMegabufferRegion<'a> {
    offset: u64,
    size: u64,
    megabuffer: Arc<Mutex<Megabuffer<'a>>>,
}

impl AllocatedMegabufferRegion {
    pub fn write<T>(&mut self, data: &[T]) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        let data = bytemuck::bytes_of(data);
        self.megabuffer
            .lock()?
            .write_bytes(data, self)
    }
}

pub struct Megabuffer<'a> {
    buffer: Buffer,
    staging_buffer: Buffer,
    free_regions: Vec<FreeMegabufferRegion>,
    mem_loc: MemoryLocation,
    alignment: u64,

    transfer_context: Arc<TransferContext<'a>>,
}

impl Megabuffer<'_> {
    pub fn new(
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_loc: MemoryLocation,
        alignment: u64,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: Arc<TransferContext>,
    ) -> Result<Arc<Mutex<Self>>> {
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

        Ok(Arc::new(Mutex::new(Self {
            buffer,
            staging_buffer,
            free_regions: vec![FreeMegabufferRegion {
                offset: 0,
                size,
            }],
            mem_loc,
            alignment,
            transfer_context,
        })))
    }

    pub fn allocate_region<'a>(
        self: &'a Arc<Mutex<Megabuffer<'a>>>,
        size: u64
    ) -> Result<AllocatedMegabufferRegion<'a>> {
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

    pub fn deallocate_region(
        self: &Arc<Mutex<Self>>,
        region: AllocatedMegabufferRegion,
    ) -> Result<()> {
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

    pub fn defragment(
        self: &Arc<Mutex<Self>>,
    ) -> Result<()> {
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

    pub fn upload(
        self: &Arc<Mutex<Self>>,
    ) -> Result<()> {
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

    pub fn write_bytes(
        &mut self,
        data: &[u8],
        region: &AllocatedMegabufferRegion,
    ) -> Result<presser::CopyRecord> {
        if (data.len() * size_of::<u8>()) as u64 > region.size {
            return Err(eyre!("Data too large for region"));
        }
        self.staging_buffer.write(data, region.offset as usize)
    }

    pub fn write<T>(
        &mut self,
        data: &[T],
        region: &AllocatedMegabufferRegion,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        if (data.len() * size_of::<T>()) as u64 > region.size {
            return Err(eyre!("Data too large for region"));
        }
        self.staging_buffer.write(data, region.offset as usize)
    }

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
