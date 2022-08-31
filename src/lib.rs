use log::info;
use std::rc::Rc;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::window::Window;
use winit::{event::Event, event_loop::EventLoop, window::WindowBuilder};

pub extern crate cgmath;
extern crate core;
pub extern crate image;

pub mod input;
use input::InputHandler;
pub mod gfx;
use gfx::Renderer3D;
use gui::GUIRenderer;

mod gui;
mod text;
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

pub struct GraphicsEngine {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,

    screen_size: PhysicalSize<u32>,

    pub renderer_3d: Renderer3D,
    pub renderer_gui: GUIRenderer,
}

impl GraphicsEngine {
    fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: Default::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Main device"),
                features: Default::default(),
                limits: Default::default(),
            },
            None,
        ))
        .unwrap();

        let screen_size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: screen_size.width,
            height: screen_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &surface_config);

        let device = Rc::new(device);
        let queue = Rc::new(queue);

        let renderer_3d = Renderer3D::new(Rc::clone(&device), Rc::clone(&queue), &surface_config);
        let renderer_gui = GUIRenderer::new(Rc::clone(&device), Rc::clone(&queue), &surface_config);

        Self {
            screen_size,
            device,
            queue,
            surface,
            surface_config,
            renderer_3d,
            renderer_gui,
        }
    }

    fn render(&self) -> anyhow::Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_pass_encoder"),
                });

        self.renderer_3d.render(&mut command_encoder, &view);
        self.renderer_gui.render(&mut command_encoder, &view);

        self.queue.submit(std::iter::once(command_encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.screen_size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.renderer_3d
                .resize(self.screen_size, &self.surface_config);
            self.renderer_gui.resize(self.screen_size);
        }
    }

    pub fn reload_view(&mut self) {
        self.resize(self.screen_size);
    }

    fn update(&mut self) {
        self.renderer_3d.update();
        self.renderer_gui.update();
    }
}

pub struct Game {
    title: String,
    game_objects: Vec<Box<dyn GameObject>>,
}

impl Game {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
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

        let mut graphics_engine = GraphicsEngine::new(&window);
        let mut input_handler = InputHandler::new();

        for go in &mut self.game_objects {
            go.start(&mut graphics_engine);
        }
        graphics_engine.update();

        let mut last_time = std::time::Instant::now();
        event_loop.run(move |event, _, control_flow| {
            // Capture result from the start function
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
                Event::RedrawRequested(_) => {
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
