use log::info;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::{event::Event, event_loop::EventLoop, window::WindowBuilder};

pub extern crate cgmath;
extern crate core;
pub extern crate image;

pub mod input;
use input::InputHandler;
pub mod gfx;
use gfx::GraphicsEngine;

pub mod util;

pub trait GameObject {
    fn start(&mut self, graphics_engine: &mut GraphicsEngine);
    fn update(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
        dt: f32,
    );
}

pub struct Game {
    title: String,
    game_objects: Vec<Box<dyn GameObject>>,
    window_width: u32,
    window_height: u32,
}

impl Game {
    pub fn new(title: &str, window_width: u32, window_height: u32) -> Self {
        Self {
            title: title.to_string(),
            game_objects: vec![],
            window_width,
            window_height,
        }
    }

    pub fn add_game_object(&mut self, go: impl 'static + GameObject) {
        self.game_objects.push(Box::new(go));
    }

    pub fn run(mut self) {
        info!("Game begins");

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(PhysicalSize::new(self.window_width, self.window_height))
            .with_resizable(false)
            .build(&event_loop)
            .unwrap();

        let mut graphics_engine = GraphicsEngine::new(&window);
        let mut input_handler = InputHandler::new();

        for go in &mut self.game_objects {
            go.start(&mut graphics_engine);
        }
        graphics_engine.update();

        let mut last_time = std::time::Instant::now();
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    input_handler.accept_input(&event);
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            graphics_engine.resize(physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            graphics_engine.resize(*new_inner_size);
                        }
                        _ => {}
                    }
                }
                Event::RedrawRequested(window_id) if window_id == window.id() => {
                    let now = std::time::Instant::now();
                    let dt = now - last_time;
                    last_time = now;

                    for go in &mut self.game_objects {
                        go.update(&mut graphics_engine, &mut input_handler, dt.as_secs_f32());
                    }
                    input_handler.update_input_state();
                    graphics_engine.update();

                    match graphics_engine.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => graphics_engine.reload_view(),
                        // The system is out of memory -> quit
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                // RedrawRequested will only trigger once, unless we manually request it
                Event::MainEventsCleared => window.request_redraw(),
                _ => {}
            }
        })
    }
}
