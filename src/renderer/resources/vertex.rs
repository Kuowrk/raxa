use glam::{Vec2, Vec3};
use crate::renderer::shader_data::PerVertexData;

#[derive(Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Vec3,
    pub texcoord: Vec2,
}

impl Vertex {
    pub fn as_shader_data(&self) -> PerVertexData {
        PerVertexData {
            position: self.position,
            texcoord: self.texcoord,
        }
    }
}