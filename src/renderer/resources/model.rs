use super::mesh::Mesh;
use super::vertex::Vertex;
use crate::renderer::core::target::RenderTarget;
use crate::renderer::internals::buffer_allocator::{BufferAllocator, BufferRegion};
use crate::renderer::shader_data::PerVertexData;
use color_eyre::eyre::{eyre, OptionExt, Result};
use glam::Vec3;

pub struct FullscreenQuad {
    quad_model: Model,
    // Image width and height determine the aspect ratio of an image to be displayed on the quad
    image_width: f32,
    image_height: f32,
}

impl FullscreenQuad {
    pub fn new(
        vertex_buffer_allocator: &mut BufferAllocator,
        index_buffer_allocator: &mut BufferAllocator,
        tgt: &RenderTarget,
    ) -> Result<Self> {
        let quad_mesh = Mesh::new_quad();
        let quad_model = Model::new(
            vec![quad_mesh],
            vertex_buffer_allocator,
            index_buffer_allocator,
        )?;
        let mut quad = Self {
            quad_model,
            // Assume a square image by default
            image_width: 1.0,
            image_height: 1.0,
        };
        quad.resize_to_target(tgt, vertex_buffer_allocator)?;
        Ok(quad)
    }

    pub fn resize_to_target(
        &mut self,
        tgt: &RenderTarget,
        vertex_buffer_allocator: &mut BufferAllocator,
    ) -> Result<()> {
        // Correct for image aspect ratio
        let mut x = if self.image_width >= self.image_height {
            1.0
        } else {
            self.image_width / self.image_height
        };
        let mut y = if self.image_width < self.image_height {
            1.0
        } else {
            self.image_height / self.image_width
        };

        // Correct for viewport aspect ratio
        let size = tgt.window.inner_size();
        if size.width >= size.height {
            y *= size.width as f32 / size.height as f32;
        } else {
            x *= size.height as f32 / size.width as f32;
        };

        // Update the vertices in the background quad vertex buffer to match the aspect ratio of background image.
        // This means that the quad may not fill the entire viewport, but the image will be displayed with the correct aspect ratio.
        // Note that only the vertex buffer gets mutated and not the vertices stored in the model themselves,
        //   meaning the model vertices can be reused to mutate the vertex buffer at a later time.
        let vertices_merged  = self.quad_model
            .get_vertices_merged()
            .iter()
            .map(|v| {
                let p = v.position;
                let mut vertex = v.as_shader_data();
                vertex.position = Vec3::new(p[0] * x, p[1] * y, p[2]);
                vertex
            })
            .collect::<Vec<PerVertexData>>();
        self.quad_model.write_vertex_buffer(vertices_merged, vertex_buffer_allocator)?;

        Ok(())
    }
}

pub struct Model {
    meshes: Vec<Mesh>,
    vertex_buffer_region: BufferRegion,
    index_buffer_region: Option<BufferRegion>,
}

impl Model {
    pub fn new(
        meshes: Vec<Mesh>,
        vertex_buffer_allocator: &mut BufferAllocator,
        index_buffer_allocator: &mut BufferAllocator,
    ) -> Result<Self> {
        if meshes.is_empty() {
            return Err(eyre!("Model must have at least one mesh"));
        }

        // Ensure that all meshes have either no indices or all indices
        let has_indices = meshes.first().unwrap().indices.is_some();
        let all_meshes_valid = if has_indices {
            meshes.iter().all(|m| m.indices.is_some())
        } else {
            meshes.iter().all(|m| m.indices.is_none())
        };
        if !all_meshes_valid {
            return Err(eyre!("All meshes must have either no indices or all indices"));
        }

        // Collect all vertices from all meshes
        let vertices = meshes
            .iter()
            .flat_map(|m| m.vertices.iter())
            .map(|v| v.as_shader_data())
            .collect::<Vec<PerVertexData>>();

        // Upload all vertices to the vertex buffer
        let vertex_buffer_region_size = (vertices.len() * size_of::<PerVertexData>()) as u64;
        let vertex_buffer_region = vertex_buffer_allocator
            .allocate(vertex_buffer_region_size)
            .ok_or_eyre("Failed to allocate vertex buffer region")?;
        vertex_buffer_allocator.write_buffer(&vertices, &vertex_buffer_region)?;

        // Upload all indices to the index buffer if the model has indices
        let index_buffer_region = if has_indices {
            // Collect all indices from all meshes
            let indices = meshes
                .iter()
                .flat_map(|m| m.indices.as_ref().unwrap().iter().cloned())
                .collect::<Vec<u32>>();

            let index_buffer_region_size = (indices.len() * size_of::<u32>()) as u64;
            let index_buffer_region = index_buffer_allocator
                .allocate(index_buffer_region_size)
                .ok_or_eyre("Failed to allocate index buffer region")?;
            index_buffer_allocator.write_buffer(&indices, &index_buffer_region)?;

            Some(index_buffer_region)
        } else {
            None
        };

        Ok(Self {
            meshes,
            vertex_buffer_region,
            index_buffer_region,
        })
    }

    pub fn write_vertex_buffer(
        &mut self,
        vertices: Vec<PerVertexData>,
        vertex_buffer_allocator: &mut BufferAllocator,
    ) -> Result<()> {
        vertex_buffer_allocator.deallocate(self.vertex_buffer_region);
        self.vertex_buffer_region = vertex_buffer_allocator
            .allocate((vertices.len() * size_of::<PerVertexData>()) as u64)
            .ok_or_eyre("Failed to allocate vertex buffer region")?;
        vertex_buffer_allocator.write_buffer(&vertices, &self.vertex_buffer_region)?;
        Ok(())
    }

    pub fn get_vertices_merged(&self) -> Vec<&Vertex> {
        self.meshes
            .iter()
            .flat_map(|m| m.vertices.iter())
            .collect()
    }

    pub fn get_indices_merged(&self) -> Option<Vec<&u32>> {
        if self.index_buffer_region.is_some() {
            Some(self.meshes
                .iter()
                .flat_map(|m| m.indices.as_ref().unwrap().iter())
                .collect())
        } else {
            None
        }
    }

    pub fn get_meshes(&self) -> &Vec<Mesh> {
        &self.meshes
    }
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.meshes
            .iter()
            .zip(other.meshes.iter())
            .all(|(self_mesh, other_mesh)| self_mesh == other_mesh)
    }
}