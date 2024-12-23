use std::sync::Arc;
use color_eyre::Result;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use super::renderer::Renderer;

pub struct App {
    renderer: Renderer,
    event_loop: EventLoop<()>,
    window: Option<Arc<Window>>,
}

impl App {
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new()?;
        let renderer = Renderer::new(&event_loop)?;
        Ok(Self {
            renderer,
            event_loop,
            window: None,
        })
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {}

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap()
        );
        self.renderer.set_window(window.clone()).unwrap();
        self.window = Some(window);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
    }
}