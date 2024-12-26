use super::mesh::Mesh;
use super::vertex::Vertex;
use crate::renderer::core::resources::RenderResources;
use crate::renderer::core::viewport::RenderViewport;
use crate::renderer::shader_data::PerVertexData;
use color_eyre::eyre::{eyre, Result};
use glam::Vec3;
use vulkano::buffer::Subbuffer;

pub struct FullscreenQuad {
    quad_model: Model,
    // Image width and height determine the aspect ratio of an image to be displayed on the quad
    image_width: f32,
    image_height: f32,
}

impl FullscreenQuad {
    pub fn new(
        res: &RenderResources,
        vpt: &RenderViewport,
    ) -> Result<Self> {
        let quad_mesh = Mesh::new_quad();
        let quad_model = Model::new(vec![quad_mesh], res)?;
        let result = Self {
            quad_model,
            // Assume a square image by default
            image_width: 1.0,
            image_height: 1.0,
        };
        result.resize_to_viewport(vpt)?;
        Ok(result)
    }

    pub fn resize_to_viewport(
        &self,
        vpt: &RenderViewport,
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
        let vpt_size = vpt.get_size();
        if vpt_size.width >= vpt_size.height {
            y *= vpt_size.width as f32 / vpt_size.height as f32;
        } else {
            x *= vpt_size.height as f32 / vpt_size.width as f32;
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
        self.quad_model.vertex_buffer.write()?.copy_from_slice(&vertices_merged);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Model {
    meshes: Vec<Mesh>,
    vertex_buffer: Subbuffer<[PerVertexData]>,
    index_buffer: Option<Subbuffer<[u32]>>,
}

impl Model {
    pub fn new(
        meshes: Vec<Mesh>,
        res: &RenderResources,
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

        // Create a GPU-side vertex buffer
        let vertex_buffer = res.vertex_buffer_allocator
            .allocate_slice::<PerVertexData>(vertices.len().into())?;
        vertex_buffer.write()?.copy_from_slice(&vertices);

        // Create a GPU-side index buffer if the model has indices
        let index_buffer = if has_indices {
            // Collect all indices from all meshes
            let indices = meshes
                .iter()
                .flat_map(|m| m.indices.as_ref().unwrap().iter().cloned())
                .collect::<Vec<u32>>();

            let index_buffer = res.index_buffer_allocator
                .allocate_slice::<u32>(indices.len().into())?;
            index_buffer.write()?.copy_from_slice(&indices);

            Some(index_buffer)
        } else {
            None
        };


        Ok(Self {
            meshes,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn get_vertices_merged(&self) -> Vec<&Vertex> {
        self.meshes
            .iter()
            .flat_map(|m| m.vertices.iter())
            .collect()
    }

    pub fn get_indices_merged(&self) -> Option<Vec<&u32>> {
        if self.index_buffer.is_some() {
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

    pub fn get_vertex_buffer(&self) -> &Subbuffer<[PerVertexData]> {
        &self.vertex_buffer
    }

    pub fn get_index_buffer(&self) -> Option<&Subbuffer<[u32]>> {
        self.index_buffer.as_ref()
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