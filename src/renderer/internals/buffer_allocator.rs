use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::Allocator;
use crate::renderer::internals::buffer::Buffer;
use crate::renderer::internals::transfer_context::TransferContext;

#[derive(PartialEq, Copy, Clone)]
pub struct BufferRegion {
    offset: u64,
    size: u64,
}

pub struct BufferAllocator<'a> {
    buffer: Buffer,
    staging_buffer: Buffer,
    free_regions: Vec<BufferRegion>,
    mem_loc: MemoryLocation,
    alignment: u64,

    transfer_context: Arc<TransferContext<'a>>,
}

impl BufferAllocator {
    pub fn new(
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_loc: MemoryLocation,
        alignment: u64,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: Arc<TransferContext>,
    ) -> Result<Self> {
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

        Ok(Self {
            buffer,
            staging_buffer,
            free_regions: vec![BufferRegion {
                offset: 0,
                size,
            }],
            mem_loc,
            alignment,

            transfer_context,
        })
    }

    pub fn allocate(&mut self, size: u64) -> Option<BufferRegion> {
        let aligned_size = (size + self.alignment - 1) & !(self.alignment - 1);
        for (i, region) in self.free_regions.iter_mut().enumerate() {
            if region.size >= aligned_size {
                let allocated_region = BufferRegion {
                    offset: region.offset,
                    size: aligned_size,
                };
                region.offset += aligned_size;
                region.size -= aligned_size;

                if region.size == 0 {
                    self.free_regions.remove(i);
                }

                return Some(allocated_region);
            }
        }

        None // No free region large enough
    }

    pub fn deallocate(&mut self, region: BufferRegion) {
        let mut left_index = None; // Some if there is a free region to the left of the deallocated region
        let mut right_index = None; // Some if there is a free region to the right of the deallocated region

        for (i, free_region) in self.free_regions.iter().enumerate() {
            if free_region.offset + free_region.size == region.offset {
                left_index = Some(i);
            } else if region.offset + region.size == free_region.offset {
                right_index = Some(i);
            }
        }

        match (left_index, right_index) {
            (Some(left), Some(right)) => {
                self.free_regions[left].size += region.size + self.free_regions[right].size;
                self.free_regions.remove(right);
            }
            (Some(left), None) => {
                self.free_regions[left].size += region.size;
            }
            (None, Some(right)) => {
                self.free_regions[right].offset = region.offset;
                self.free_regions[right].size += region.size;
            }
            (None, None) => {
                self.free_regions.push(region);
                self.free_regions.sort_by_key(|r| r.offset);
            }
        }
    }

    pub fn defragment(&mut self) {
        self.free_regions.sort_by_key(|r| r.offset);

        // Merge adjacent free regions
        let mut i = 0;
        while i < self.free_regions.len() - 1 {
            if self.free_regions[i].offset + self.free_regions[i].size == self.free_regions[i + 1].offset {
                self.free_regions[i].size += self.free_regions[i + 1].size;
                self.free_regions.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    pub fn update_buffer(&self) -> Result<()> {
        self.transfer_context.immediate_submit(
            |cmd: vk::CommandBuffer, device: &ash::Device| {
                let copy_regions = self.free_regions.iter().map(|region| {
                    vk::BufferCopy {
                        src_offset: region.offset,
                        dst_offset: region.offset,
                        size: region.size,
                    }
                }).collect::<Vec<_>>();

                unsafe {
                    device.cmd_copy_buffer(
                        cmd,
                        self.staging_buffer.buffer,
                        self.buffer.buffer,
                        &copy_regions,
                    );
                }

                Ok(())
            },
        )?;

        Ok(())
    }

    pub fn write_buffer<T>(
        &mut self,
        data: &[T],
        region: &BufferRegion,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        if (data.len() * size_of::<T>()) as u64 > region.size {
            return Err(eyre!("Data too large for region"));
        }
        self.staging_buffer.write(data, region.offset as usize)
    }
}