use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
struct PerFrameData {
    pub viewproj: Mat4,
    pub near: f32,
    pub far: f32,
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
struct PerMaterialData {
    texture_index: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
struct PerObjectData {
    model: Mat4,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
struct PerVertexData {
    position: Vec3,
    texcoord: Vec2,
}

/// Data that is unique to each draw call passed as a push constant
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
struct PerDrawData {
    object_index: u32,
    material_index: u32,
    vertex_offset: u32,
}
