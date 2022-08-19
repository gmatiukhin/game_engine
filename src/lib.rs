use log::{info, warn};
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::{event::Event, event_loop::EventLoop, window::WindowBuilder};

pub extern crate cgmath;

pub mod input;
use input::InputHandler;
pub mod gfx;
use gfx::Renderer;

pub trait GameObject {
    fn start(&mut self, renderer: &mut Renderer);
    fn update(
        &mut self,
        renderer: &mut Renderer,
        input_handler: &mut InputHandler,
        dt: std::time::Duration,
    );
}

pub struct Game {
    title: String,
    game_objects: Vec<Box<dyn GameObject>>,
}

impl Game {
    pub fn new(title: String) -> Self {
        Self {
            title,
            game_objects: vec![],
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
            .build(&event_loop)
            .unwrap();

        let mut renderer = pollster::block_on(Renderer::new(&window));
        let mut input_handler = InputHandler::new();

        for go in &mut self.game_objects {
            go.start(&mut renderer);
        }
        renderer.init();

        let mut last_time = std::time::Instant::now();
        event_loop.run(move |event, _, control_flow| {
            // Capture result from the start function
            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    input_handler.accept_input(&event);
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            renderer.resize(physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            renderer.resize(*new_inner_size);
                        }
                        _ => {}
                    }
                }
                Event::RedrawRequested(_) => {
                    let now = std::time::Instant::now();
                    let dt = now - last_time;
                    last_time = now;

                    for go in &mut self.game_objects {
                        go.update(&mut renderer, &mut input_handler, dt);
                    }
                    input_handler.reset_scroll();

                    match renderer.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => renderer.reload_view(),
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
