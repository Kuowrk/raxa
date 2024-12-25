use glam::Vec2;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

#[derive(Default)]
pub struct InputState {
    pub mouse_curr_pos: Vec2,
    pub mouse_prev_pos: Vec2,
    pub mouse_wheel_delta_y: f32,
    pub mouse_left_down: bool,

    pub mouse_right_just_pressed: bool,
    pub mouse_right_just_released: bool,
    pub mouse_right_down: bool,
    pub mouse_right_just_pressed_pos: Vec2,
    pub mouse_right_just_released_pos: Vec2,
    pub mouse_just_left: bool,
    pub mouse_just_entered: bool,
}

impl InputState {
    pub fn process_window_events(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        self.mouse_right_just_pressed = true;
                        self.mouse_right_just_released = false;
                        self.mouse_right_down = true;
                        self.mouse_right_just_pressed_pos = self.mouse_curr_pos;
                    }
                    ElementState::Released => {
                        self.mouse_right_just_pressed = false;
                        self.mouse_right_just_released = true;
                        self.mouse_right_down = false;
                        self.mouse_right_just_released_pos = self.mouse_curr_pos;
                    }
                }
            }
            WindowEvent::CursorMoved {
                position,
                ..
            } => {
                self.mouse_prev_pos = self.mouse_curr_pos;
                self.mouse_curr_pos = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel {
                delta,
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_x, y) => {
                        self.mouse_wheel_delta_y = y.signum();
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        self.mouse_wheel_delta_y = pos.y.signum() as f32;
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse_just_left = true;
            }
            WindowEvent::CursorEntered { .. } => {
                self.mouse_just_entered = true;
            }
            _ => {}
        }
    }

    /// Reset the input states for the next frame.
    pub fn reset_frame(&mut self) {
        self.mouse_wheel_delta_y = 0.0;
        self.mouse_prev_pos = self.mouse_curr_pos;
        self.mouse_right_just_pressed = false;
        self.mouse_right_just_released = false;
        self.mouse_just_left = false;
        self.mouse_just_entered = false;
    }
}