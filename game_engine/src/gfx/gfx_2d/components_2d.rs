use crate::gfx::texture;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

pub enum GUITransform {
    /// Pixel values
    Absolute(u32, u32),
    /// As a percentage of the corresponding parent transform
    Relative(f32, f32),
}

pub enum GUIPanelContent {
    Image(texture::Image),
    Text(super::text::TextParameters),
    Panels(wgpu::Color, Vec<GUIPanel>),
    Surface2D(Surface2D),
}

pub struct GUIPanel {
    pub name: String,
    pub active: bool,
    /// Position of the top-left corner of the panel
    pub position: GUITransform,
    pub dimensions: GUITransform,

    pub content: GUIPanelContent,
}

impl GUIPanel {
    pub(super) fn buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        text_rasterizer: &super::text::TextRasterizer,
        parent_anchor: cgmath::Vector2<f32>,
        parent_dimensions: PhysicalSize<f32>,
    ) -> Option<GUIPanelBuffered> {
        if !self.active {
            return None;
        }

        let (left, top) = match self.position {
            GUITransform::Absolute(x, y) => {
                (parent_anchor.x + x as f32, parent_anchor.y + y as f32)
            }
            GUITransform::Relative(percentage_x, percentage_y) => (
                parent_anchor.x + parent_dimensions.width as f32 * percentage_x,
                parent_anchor.y + parent_dimensions.height as f32 * percentage_y,
            ),
        };

        let (right, bottom) = match self.dimensions {
            GUITransform::Absolute(width, height) => (left + width as f32, top + height as f32),
            GUITransform::Relative(percentage_x, percentage_y) => (
                left + parent_dimensions.width as f32 * percentage_x,
                top + parent_dimensions.height as f32 * percentage_y,
            ),
        };

        let left = left
            .max(parent_anchor.x)
            .min(parent_dimensions.width + parent_anchor.x);
        let top = top
            .max(parent_anchor.y)
            .min(parent_dimensions.height + parent_anchor.y);
        let right = right
            .max(parent_anchor.x)
            .min(parent_dimensions.width + parent_anchor.x);
        let bottom = bottom
            .max(parent_anchor.y)
            .min(parent_dimensions.height + parent_anchor.y);

        let vertices = vec![
            // Top left
            GUIVertex {
                position: [left, top],
                text_coords: [0.0, 0.0],
            },
            // Bottom left
            GUIVertex {
                position: [left, bottom],
                text_coords: [0.0, 1.0],
            },
            // Bottom right
            GUIVertex {
                position: [right, bottom],
                text_coords: [1.0, 1.0],
            },
            // Top right
            GUIVertex {
                position: [right, top],
                text_coords: [1.0, 0.0],
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gui_vertex_buffer"),
            contents: &bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gui_index_buffer"),
            contents: &bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (texture, children) = match &mut self.content {
            GUIPanelContent::Image(img) => (
                texture::Texture::from_image(device, queue, &img.file, &img.name),
                vec![],
            ),
            GUIPanelContent::Text(text) => {
                let width: u32 = (right - left) as u32;
                let height: u32 = (bottom - top) as u32;
                let data = text_rasterizer.get_rgba_from_text(text, width, height);
                (
                    texture::Texture::from_bytes_rgba(device, queue, data, width, height),
                    vec![],
                )
            }
            GUIPanelContent::Panels(color, children) => {
                let mut buffered_children: Vec<GUIPanelBuffered> = vec![];
                for child in children {
                    if let Some(panel_buffered) = child.buffer(
                        &device,
                        &queue,
                        &texture_bind_group_layout,
                        text_rasterizer,
                        (left, top).into(),
                        (right - left, bottom - top).into(),
                    ) {
                        buffered_children.push(panel_buffered);
                    }
                }

                (
                    texture::Texture::from_color(device, queue, color),
                    buffered_children,
                )
            }
            GUIPanelContent::Surface2D(surface) => {
                if let Some(image) = surface.image((right - left, bottom - top).into()) {
                    (
                        texture::Texture::from_image(&device, &queue, &image, "Surface image"),
                        vec![],
                    )
                } else {
                    (texture::Texture::default_texture(&device, &queue), vec![])
                }
            }
        };

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("panel"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        Some(GUIPanelBuffered {
            vertex_buffer,
            index_buffer,
            indices_len: indices.len() as u32,
            texture_bind_group,
            children,
        })
    }
}

#[derive(Debug)]
pub(super) struct GUIPanelBuffered {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    indices_len: u32,
    texture_bind_group: wgpu::BindGroup,
    children: Vec<GUIPanelBuffered>,
}

impl GUIPanelBuffered {
    pub(super) fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.indices_len, 0, 0..1);

        for child in &self.children {
            child.render(render_pass);
        }
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone, Debug)]
pub(super) struct GUIVertex {
    pub(super) position: [f32; 2],
    /// In wgpu's coordinate system UV origin is situated in the top left corner
    pub(super) text_coords: [f32; 2],
}

impl GUIVertex {
    pub(super) fn format<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

pub struct Surface2D {
    width: u32,
    height: u32,
    background_color: wgpu::Color,
    cells: Vec<wgpu::Color>,
    is_updated: bool,
    image: Option<image::DynamicImage>,
}

impl Surface2D {
    pub fn new(width: u32, height: u32, background_color: wgpu::Color) -> Self {
        Self {
            width,
            height,
            background_color,
            cells: vec![background_color; (width * height) as usize],
            is_updated: true,
            image: None,
        }
    }

    pub fn set_cell_color(&mut self, x: u32, y: u32, color: wgpu::Color) {
        self.cells[(y * self.width + x) as usize] = color;
        self.is_updated = true;
    }

    pub fn clear(&mut self) {
        for el in self.cells.iter_mut() {
            *el = self.background_color;
        }
        self.is_updated = true;
    }
}

impl Surface2D {
    pub(super) fn image(
        &mut self,
        panel_dimensions: PhysicalSize<f32>,
    ) -> Option<image::DynamicImage> {
        if let Some(image) = &self.image {
            Some(image.clone())
        } else {
            if self.is_updated {
                let cell_width = panel_dimensions.width as u32 / self.width;
                let cell_height = panel_dimensions.height as u32 / self.height;

                let texture_width = self.width * cell_width;
                let texture_height = self.height * cell_height;

                let mut image_buffer = image::ImageBuffer::new(texture_width, texture_height);

                for x in 0..texture_width {
                    for y in 0..texture_height {
                        let cells_index = y / cell_height * self.width + x / cell_width;
                        let color: wgpu::Color = self.cells[cells_index as usize];

                        image_buffer.put_pixel(
                            x,
                            y,
                            [
                                (color.r * 255.0) as u8,
                                (color.g * 255.0) as u8,
                                (color.b * 255.0) as u8,
                                (color.a * 255.0) as u8,
                            ]
                            .into(),
                        );
                    }
                }
                let image = image::DynamicImage::ImageRgba8(image_buffer);
                self.image = Some(image.clone());

                Some(image)
            } else {
                None
            }
        }
    }
}
