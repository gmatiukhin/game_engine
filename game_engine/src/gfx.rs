use crate::gfx::gfx_2d::Renderer2D;
use crate::gfx::gfx_3d::Renderer3D;
use log::info;
use std::rc::Rc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub mod gfx_2d;
pub mod gfx_3d;
pub mod texture;

pub struct GraphicsEngine {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,

    screen_size: PhysicalSize<u32>,

    pub renderer_3d: Renderer3D,
    pub renderer_2d: Renderer2D,
}

impl GraphicsEngine {
    pub(super) fn new(window: &Window) -> Self {
        info!("Creating GraphicsEngine");
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
        let renderer_gui = Renderer2D::new(Rc::clone(&device), Rc::clone(&queue), &surface_config);

        Self {
            screen_size,
            device,
            queue,
            surface,
            surface_config,
            renderer_3d,
            renderer_2d: renderer_gui,
        }
    }

    pub(super) fn render(&self) -> anyhow::Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_pass_encoder"),
                });

        self.renderer_2d
            .render_background(&mut command_encoder, &view);
        self.renderer_3d.render_scene(&mut command_encoder, &view);
        self.renderer_2d
            .render_foreground(&mut command_encoder, &view);

        self.queue.submit(std::iter::once(command_encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    pub(super) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.screen_size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.renderer_3d
                .resize(self.screen_size, &self.surface_config);
            self.renderer_2d.resize(self.screen_size);
        }
    }

    pub(super) fn reload_view(&mut self) {
        self.resize(self.screen_size);
    }

    pub(super) fn update(&mut self) {
        self.renderer_3d.update();
        self.renderer_2d.update();
    }
}
