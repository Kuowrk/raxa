use crate::renderer::util;
use glam::{Mat4, Vec3};

pub struct Camera {
    position: Vec3,
    forward: Vec3,
    up: Vec3,
    right: Vec3,
    world_up: Vec3,
    fov_y_deg: f32,
    near: f32,
    far: f32,
    pivot: Vec3,
}

impl Camera {
    const DEFAULT_FOV_Y_DEG: f32 = 45.0;

    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            right: Vec3::X,
            world_up: Vec3::Y,
            fov_y_deg: Self::DEFAULT_FOV_Y_DEG,
            near: 0.1,
            far: 100.0,
            pivot: Vec3::ZERO,
        }
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.look_at(self.pivot);
    }

    pub fn look_at(&mut self, target: Vec3) {
        if target == self.position {
            return;
        }
        self.pivot = target;
        self.forward = (target - self.position).normalize();
        self.right = self.forward.cross(self.world_up).normalize();
        self.up = self.right.cross(self.forward).normalize();
    }

    pub fn get_viewproj_mat(
        &self,
        window: &winit::window::Window,
    ) -> Mat4 {
        self.get_proj_mat(window) * self.get_view_mat()
    }

    pub fn get_view_mat(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward, self.up)
    }

    pub fn get_proj_mat(
        &self,
        window: &winit::window::Window,
    ) -> Mat4 {
        let size = window.inner_size();
        let aspect_ratio = size.width as f32 / size.height as f32;
        Mat4::perspective_rh(
            self.fov_y_deg.to_radians(),
            aspect_ratio,
            self.near,
            self.far,
        )
    }

    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    pub fn get_forward(&self) -> Vec3 {
        self.forward
    }

    pub fn get_up(&self) -> Vec3 {
        self.up
    }

    pub fn get_right(&self) -> Vec3 {
        self.right
    }

    pub fn get_near(&self) -> f32 {
        self.near
    }

    pub fn get_far(&self) -> f32 {
        self.far
    }

    pub fn get_pivot(&self) -> Vec3 {
        self.pivot
    }

    pub fn get_world_up(&self) -> Vec3 {
        self.world_up
    }

    pub fn get_pitch(&self) -> f32 {
        util::calculate_pitch(self.forward)
    }
}
