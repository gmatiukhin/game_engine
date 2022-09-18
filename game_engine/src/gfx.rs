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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
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
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_pass_encoder"),
                });

        // Surface texture is of BGRA format
        let q_background = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: self.surface_config.width,
                height: self.surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
        });

        let mut data = vec![0; (self.screen_size.width * self.screen_size.height * 4) as usize];
        // Todo: write background values to `data`

        self.queue.write_texture(
            q_background.as_image_copy(),
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * self.screen_size.width),
                rows_per_image: std::num::NonZeroU32::new(1 * self.screen_size.height),
            },
            wgpu::Extent3d {
                width: self.screen_size.width,
                height: self.screen_size.height,
                depth_or_array_layers: 1,
            },
        );

        self.renderer_3d.render_scene(
            &mut command_encoder,
            &q_background.create_view(&wgpu::TextureViewDescriptor::default()),
        );

        let aligned_bytes_per_row = 4 * self.screen_size.width
            + (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
                - 4 * self.screen_size.width % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
                % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let storage_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (aligned_bytes_per_row * self.screen_size.height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let img_storage_buffer = wgpu::ImageCopyBuffer {
            buffer: &storage_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(aligned_bytes_per_row),
                rows_per_image: std::num::NonZeroU32::new(self.screen_size.height),
            },
        };

        command_encoder.copy_texture_to_buffer(
            q_background.as_image_copy(),
            img_storage_buffer,
            wgpu::Extent3d {
                width: self.screen_size.width,
                height: self.screen_size.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(command_encoder.finish()));

        storage_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |res| match res {
                Ok(_) => {}
                Err(err) => eprintln!("{}", err),
            });

        self.device.poll(wgpu::Maintain::Wait);

        let data = storage_buffer.slice(..).get_mapped_range().to_vec();
        // Todo: write foreground values to `data`

        self.queue.write_texture(
            surface_texture.texture.as_image_copy(),
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(aligned_bytes_per_row),
                rows_per_image: std::num::NonZeroU32::new(self.screen_size.height),
            },
            wgpu::Extent3d {
                width: self.surface_config.width,
                height: self.surface_config.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::empty());

        surface_texture.present();
        storage_buffer.unmap();

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
            //self.renderer_2d.resize(self.screen_size);
        }
    }

    pub(super) fn reload_view(&mut self) {
        self.resize(self.screen_size);
    }

    pub(super) fn update(&mut self) {
        self.renderer_3d.update();
        //self.renderer_2d.update();
    }
}
