use std::f32::consts::PI;
use glam::{FloatExt, Mat4, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
use winit::dpi::PhysicalPosition;
use winit::error::ExternalError;
use winit::window::Window;
use crate::app::input_state::InputState;
use crate::renderer::camera::{calculate_direction, calculate_pitch, calculate_yaw, Camera};

pub struct CameraController {
    camera: Camera,

    rotation_sensitivity: f32,
    rotation_smoothing_speed: f32,
    rotation_desired_pivot_to_eye: Vec3,
    rotation_current_pivot_to_eye: Vec3,
    rotation_max_angle_y: f32,

    zoom_sensitivity: f32,
    zoom_smoothing_speed: f32,
    zoom_desired_distance: f32,
    zoom_current_distance: f32,
}

impl CameraController {
    pub fn new(camera: Camera) -> Self {
        let zoom_current_distance = camera.get_pivot().distance(camera.get_position());
        let rotation_current_pivot_to_eye = camera.get_position() - camera.get_pivot();
        Self {
            camera,

            rotation_sensitivity: 2.0,
            rotation_smoothing_speed: 10.0,
            rotation_desired_pivot_to_eye: rotation_current_pivot_to_eye,
            rotation_current_pivot_to_eye,
            rotation_max_angle_y: 80.0_f32.to_radians(),

            zoom_sensitivity: 2.0,
            zoom_smoothing_speed: 4.0,
            zoom_desired_distance: zoom_current_distance,
            zoom_current_distance,
        }
    }

    pub fn get_camera(&self) -> &Camera {
        &self.camera
    }

    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn process_input(
        &mut self,
        input_state: &mut InputState,
        window: &Window,
        delta_time: f32,
    ) {
        let window_size = window.inner_size();
        let window_center = Vec2::new(
            window_size.width as f32 / 2.0,
            window_size.height as f32 / 2.0,
        );

        {
            if input_state.mouse_right_just_pressed {
                window.set_cursor_visible(false);
                // Set the cursor position to the center of the viewport
                self.set_window_mouse_pos(window, window_center);
                input_state.mouse_curr_pos = window_center;
                input_state.mouse_prev_pos = input_state.mouse_curr_pos;
            }
            else if input_state.mouse_right_just_released {
                window.set_cursor_visible(true);
                // Reset the cursor position to the position where the right mouse button was pressed
                self.set_window_mouse_pos(window, input_state.mouse_right_just_pressed_pos);
                input_state.mouse_curr_pos = input_state.mouse_right_just_pressed_pos;
                input_state.mouse_prev_pos = input_state.mouse_curr_pos;
            }
        }

        if input_state.mouse_right_down {
            self.set_desired_rotation_pivot_to_eye(
                input_state.mouse_prev_pos,
                input_state.mouse_curr_pos,
                window_size.width as f32,
                window_size.height as f32,
            );

            if self.mouse_just_left_border(
                input_state,
                window,
                window_size.width.min(window_size.height) / 4
            ) {
                let prev_to_curr = input_state.mouse_curr_pos - input_state.mouse_prev_pos;
                input_state.mouse_prev_pos = window_center;
                input_state.mouse_curr_pos = input_state.mouse_prev_pos + prev_to_curr;
                self.set_window_mouse_pos(window, input_state.mouse_curr_pos);
            }
        }

        self.set_desired_zoom_distance(input_state.mouse_wheel_delta_y * self.zoom_sensitivity);

        self.update_zoom_lerp(delta_time);
        self.update_rotation_slerp(delta_time);
    }

    fn set_desired_zoom_distance(&mut self, delta: f32) {
        if delta == 0.0 {
            return;
        }

        let cam = &self.camera;
        let cam_near = cam.get_near();
        let cam_far = cam.get_far();

        // Scale delta by the current distance to make zooming speed independent of distance
        let delta = delta * self.zoom_current_distance * 0.1;
        let new_distance = (self.zoom_current_distance - delta)
            .max(cam_near + 0.1)
            .min(cam_far - 0.1);
        self.zoom_desired_distance = new_distance;
    }

    fn set_desired_rotation_pivot_to_eye(
        &mut self,
        prev_mouse_pos: Vec2,
        curr_mouse_pos: Vec2,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        let cam = &self.camera;

        // Calculate the amount of rotation given the mouse movement
        let delta_angle_x = 2.0 * PI / viewport_width; // Left to right = 2*PI = 360deg
        let delta_angle_y = PI / viewport_height; // Top to bottom = PI = 180deg
        let angle_x = (prev_mouse_pos.x - curr_mouse_pos.x) * delta_angle_x * self.rotation_sensitivity;
        let angle_y = (prev_mouse_pos.y - curr_mouse_pos.y) * delta_angle_y * self.rotation_sensitivity;

        if angle_x == 0.0 && angle_y == 0.0 {
            return;
        }

        // Rotate the camera around the pivot point on the up axis
        let rot_x = Mat4::from_axis_angle(cam.get_up(), angle_x);

        // Rotate the camera around the pivot point on the right axis
        let rot_y = Mat4::from_axis_angle(cam.get_right(), angle_y);

        // Set the desired pivot to eye vector
        let v = &self.rotation_current_pivot_to_eye;
        let curr_piv_to_eye = Vec4::new(v.x, v.y, v.z, 1.0);
        let new_piv_to_eye = (rot_x * rot_y * curr_piv_to_eye).xyz();

        if calculate_pitch(new_piv_to_eye).abs() <= self.rotation_max_angle_y {
            self.rotation_desired_pivot_to_eye = new_piv_to_eye;
        }
        else {
            // Clamp the pitch angle
            let pitch = self.rotation_max_angle_y * new_piv_to_eye.y.signum();
            let yaw = calculate_yaw(new_piv_to_eye);
            let new_piv_to_eye = calculate_direction(pitch, yaw);
            self.rotation_desired_pivot_to_eye = new_piv_to_eye;
        }
    }

    fn update_rotation_slerp(&mut self, delta_time: f32) {
        let t = 1.0 - (-self.rotation_smoothing_speed * delta_time).exp();
        //let t = self.rotation_smoothing_speed * delta_time;
        self.rotation_current_pivot_to_eye = slerp(
            self.rotation_current_pivot_to_eye,
            self.rotation_desired_pivot_to_eye,
            t,
        ) * self.zoom_current_distance;
        self.camera.set_position(self.camera.get_pivot() + self.rotation_current_pivot_to_eye);
    }

    fn update_zoom_lerp(&mut self, delta_time: f32) {
        let t = 1.0 - (-self.zoom_smoothing_speed * delta_time).exp();
        //let t = self.zoom_smoothing_speed * delta_time;
        self.zoom_current_distance = self.zoom_current_distance.lerp(
            self.zoom_desired_distance,
            t,
        );
        self.camera.set_position(self.camera.get_pivot() - self.camera.get_forward() * self.zoom_current_distance);
    }

    fn set_window_mouse_pos(
        &mut self,
        window: &Window,
        pos: Vec2,
    ) {
        window
            .set_cursor_position(PhysicalPosition::new(
                pos.x as f64,
                pos.y as f64,
            ))
            .or_else(|e| {
                log::error!("Failed to set cursor position: {e}");
                Ok::<(), ExternalError>(())
            })
            .unwrap();
    }

    fn mouse_just_left_border(
        &self,
        input_state: &InputState,
        window: &Window,
        border_px: u32,
    ) -> bool {
        let window_size = window.inner_size();
        let pos = input_state.mouse_curr_pos;
        pos.x < border_px as f32
            || pos.y < border_px as f32
            || pos.x > window_size.width as f32 - border_px as f32
            || pos.y > window_size.height as f32 - border_px as f32
    }

}

#[allow(dead_code)]
fn slerp_2d(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    slerp(Vec3::new(a.x, a.y, 0.0), Vec3::new(b.x, b.y, 0.0), t).xy()
}

#[allow(dead_code)]
fn slerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    // Ensure the vectors are normalized
    let a = a.normalize();
    let b = b.normalize();

    // Compute the angle between a and b
    let dot = a.dot(b).clamp(-1.0, 1.0); // Clamp to avoid numerical errors
    let theta = dot.acos();

    // If the angle is very small, fallback to LERP (avoids division by 0)
    if theta.abs() < 1e-6 {
        return a.lerp(b, t).normalize();
    }

    // SLERP formula
    let sin_theta = theta.sin();
    let a_part = (((1.0 - t) * theta).sin() / sin_theta) * a;
    let b_part = ((t * theta).sin() / sin_theta) * b;

    a_part + b_part
}