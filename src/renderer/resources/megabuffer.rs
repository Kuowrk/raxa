use crate::renderer::resources::buffer::Buffer;
use crate::renderer::contexts::device_ctx::transfer_ctx::TransferContext;
use ash::vk;
use color_eyre::eyre::{eyre, OptionExt};
use color_eyre::Result;
use gpu_allocator::vulkan::Allocator;
use gpu_allocator::MemoryLocation;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

static MEGABUFFER_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[repr(transparent)]
pub struct Megabuffer(Arc<Mutex<MegabufferInner>>);

impl Clone for Megabuffer {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq for Megabuffer {
    fn eq(&self, other: &Self) -> bool {
        self.0.lock().unwrap().id == other.0.lock().unwrap().id
    }
}

pub struct FreeMegabufferRegion {
    offset: u64,
    size: u64,
}

pub struct AllocatedMegabufferRegion{
    offset: u64,
    size: u64,
    megabuffer: Option<Megabuffer>,
}

impl AllocatedMegabufferRegion {
    pub fn write<T>(&mut self, data: &[T]) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        self.megabuffer.as_ref().unwrap().write(data, self)
    }

    pub fn suballocate_region(&mut self, size: u64) -> Result<AllocatedMegabufferRegion> {
        if size > self.size {
            return Err(eyre!("Subregion size too large"));
        }
        if size == 0 {
            return Err(eyre!("Subregion size cannot be zero"));
        }
        if size == self.size {
            return Err(eyre!("Subregion size cannot be the parent region"));
        }
        
        let subregion = AllocatedMegabufferRegion {
            offset: self.offset + (self.size - size),
            size,
            megabuffer: self.megabuffer.clone(),
        };
        self.size -= size;

        Ok(subregion)
    }

    pub fn combine_adjacent_region(&mut self, other: Self) -> Result<()> {
        if self.megabuffer != other.megabuffer {
            return Err(eyre!("Cannot combine regions belonging to different megabuffers"));
        }

        let (
            new_offset,
            new_size,
        ) = {
            let (
                left_offset,
                left_size,
                right_offset,
                right_size,
            ) = if self.offset < other.offset {
                (self.offset, self.size, other.offset, other.size)
            } else {
                (other.offset, other.size, self.offset, self.size)
            };

            let adjacent = left_offset + left_size == right_offset;
            if !adjacent {
                return Err(eyre!("Cannot combine regions that are not adjacent"));
            }

            let new_offset = left_offset;
            let new_size = left_size + right_size;

            (new_offset, new_size)
        };

        self.offset = new_offset;
        self.size = new_size;

        Ok(())
    }
}

impl Drop for AllocatedMegabufferRegion {
    fn drop(&mut self) {
        let megabuffer = self.megabuffer.take().unwrap();
        megabuffer.deallocate_region(self).unwrap();
    }
}

struct MegabufferInner {
    buffer: Buffer,
    staging_buffer: Buffer,
    free_regions: Vec<FreeMegabufferRegion>,
    mem_loc: MemoryLocation,
    alignment: u64,

    transfer_context: Arc<TransferContext>,
    id: usize,
}

pub trait MegabufferExt<B> {
    fn new(
        size: u64,
        usage: vk::BufferUsageFlags,
        alignment: u64,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: Arc<TransferContext>,
    ) -> Result<B>;
    fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion>;
    fn deallocate_region(&self, region: &mut AllocatedMegabufferRegion) -> Result<()>;
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

impl MegabufferExt<Megabuffer> for Megabuffer {
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

        let id = MEGABUFFER_ID_COUNTER
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

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
            id,
        }))))
    }

    fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let mut guard = self.0
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
            megabuffer: Some(self.clone()),
        };

        Ok(allocated_region)
    }

    fn deallocate_region(&self, region: &mut AllocatedMegabufferRegion) -> Result<()> {
        if region.size == 0 {
            return Err(eyre!("Cannot deallocate region with size 0"));
        }
        if self != region.megabuffer.as_ref().unwrap() {
            return Err(eyre!("Cannot deallocate region belonging to different megabuffer"));
        }
        
        let mut guard = self.0
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

        region.size = 0;

        Ok(())
    }

    fn defragment(&self) -> Result<()> {
        let mut guard = self.0
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
        let guard = self.0
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

        let mut guard = self.0
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

impl PartialEq for MegabufferInner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
