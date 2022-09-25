use log::info;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::window::Fullscreen;
use winit::{event::Event, event_loop::EventLoop, window::WindowBuilder};

pub extern crate cgmath;
extern crate core;
pub extern crate image;

pub mod input;
use input::InputHandler;
pub mod gfx;
use gfx::GraphicsEngine;

pub mod util;

#[allow(unused_variables)]
pub trait GameObject {
    fn start(&mut self, game_state: &mut GameState, graphics_engine: &mut GraphicsEngine) {}

    fn update(
        &mut self,
        game_state: &mut GameState,
        graphics_engine: &mut GraphicsEngine,
        input_handler: &mut InputHandler,
    );

    fn end(&mut self) {}
}

pub struct GameState {
    frame_size: PhysicalSize<u32>,
    fps: u32,
    dt: f32,
    exit: bool,
}

impl GameState {
    pub fn frame_size(&self) -> PhysicalSize<u32> {
        self.frame_size
    }

    pub fn fps(&self) -> u32 {
        self.fps
    }

    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeMode {
    KeepAspectRatio,
    Resize,
    NoResize,
    Fullscreen,
}

#[derive(Debug, Copy, Clone)]
pub struct WindowSettings {
    pub logical_width: u32,
    pub logical_height: u32,
    pub resize_mode: ResizeMode,
}

pub struct Game {
    title: String,
    game_objects: Vec<Box<dyn GameObject>>,
    window_settings: WindowSettings,
}

impl Game {
    pub fn new(title: &str, window_settings: WindowSettings) -> Self {
        Self {
            title: title.to_string(),
            game_objects: vec![],
            window_settings,
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
            .with_inner_size(PhysicalSize::new(
                self.window_settings.logical_width,
                self.window_settings.logical_height,
            ));

        let window = match self.window_settings.resize_mode {
            ResizeMode::NoResize => window.with_resizable(false),
            ResizeMode::Resize => window.with_resizable(true),
            ResizeMode::KeepAspectRatio => window.with_resizable(true),
            ResizeMode::Fullscreen => window.with_fullscreen(Some(Fullscreen::Borderless(None))),
        }
        .build(&event_loop)
        .unwrap();

        let mut graphics_engine = GraphicsEngine::new(&window, self.window_settings);
        let mut input_handler = InputHandler::new();

        let mut game_state = GameState {
            frame_size: PhysicalSize::new(
                self.window_settings.logical_width,
                self.window_settings.logical_height,
            ),
            fps: 0,
            dt: 0.0,
            exit: false,
        };

        for go in &mut self.game_objects {
            go.start(&mut game_state, &mut graphics_engine);
        }

        if game_state.exit {
            self.call_end();
            return;
        }

        graphics_engine.update();

        let mut last_time = std::time::Instant::now();
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    input_handler.accept_input(&event);
                    match event {
                        WindowEvent::CloseRequested => {
                            self.call_end();
                            *control_flow = ControlFlow::Exit
                        }
                        WindowEvent::Resized(physical_size) => {
                            game_state.frame_size = physical_size;
                            graphics_engine.resize(physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            game_state.frame_size = *new_inner_size;
                            graphics_engine.resize(*new_inner_size);
                        }
                        _ => {}
                    }
                }
                Event::RedrawRequested(window_id) if window_id == window.id() => {
                    let now = std::time::Instant::now();
                    let dt = now - last_time;
                    last_time = now;
                    game_state.fps = (std::time::Duration::from_secs(1) / dt.as_nanos() as u32)
                        .as_nanos() as u32;
                    game_state.dt = dt.as_secs_f32();
                    println!("FPS: {}", game_state.fps);

                    for go in &mut self.game_objects {
                        go.update(&mut game_state, &mut graphics_engine, &mut input_handler);
                    }

                    if game_state.exit {
                        self.call_end();
                        *control_flow = ControlFlow::Exit;
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

    fn call_end(&mut self) {
        for go in &mut self.game_objects {
            go.end();
        }
    }
}
