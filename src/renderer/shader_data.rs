use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3, Vec4};

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
    color: Vec4,
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
