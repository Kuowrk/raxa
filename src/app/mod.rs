mod input_state;
mod camera_controller;

use super::renderer::Renderer;
use color_eyre::Result;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};
use crate::app::camera_controller::CameraController;
use crate::app::input_state::InputState;
use crate::renderer::camera::Camera;

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    event_loop: EventLoop<()>,
    camera_controller: CameraController,

    // State
    input_state: InputState,
    prev_frame_time: Instant,
    delta_time_secs: f32,
    request_redraws: bool,
    close_requested: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new()?;
        let camera = Camera::new();
        let camera_controller = CameraController::new(camera);

        Ok(Self {
            window: None,
            renderer: None,
            event_loop,
            camera_controller,

            input_state: InputState::default(),
            prev_frame_time: Instant::now(),
            delta_time_secs: 0.0,
            request_redraws: false,
            close_requested: false,
        })
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        let curr_frame_time = Instant::now();
        self.delta_time_secs = curr_frame_time.duration_since(self.prev_frame_time).as_secs_f32();
        self.prev_frame_time = curr_frame_time;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window = Some(Arc::new(
                event_loop
                    .create_window(Window::default_attributes())
                    .unwrap()
            ));
        }

        if self.renderer.is_none() {
            self.renderer = Some(Renderer::new(&self.event_loop, self.window.clone()).unwrap());
        }
    }

    /*
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
    }
     */

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent
    ) {
        if window_id != self.window.as_ref().unwrap().id() {
            return;
        }

        self.input_state.process_window_events(&event);

        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::Resized(_new_size) => {
                self.renderer.request_resize();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.renderer.request_resize();
            }
            WindowEvent::RedrawRequested => {
                self.renderer.draw().unwrap();
            }
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    logical_key: key,
                    state: ElementState::Pressed,
                    ..
                },
                ..
            } => match key.as_ref() {
                Key::Character("r") => {
                    self.request_redraws = !self.request_redraws;
                    log::info!("request_redraws: {}", self.request_redraws);
                }
                Key::Named(NamedKey::Escape) => {
                    self.close_requested = true;
                }
                _ => {}
            },
            _ => {}
        }
    }

    /*
    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
    }
     */

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.request_redraws {
            self.window.as_ref().unwrap().request_redraw();
        }

        if self.close_requested {
            event_loop.exit();
        }
    }

    /*
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
    }
     */
}