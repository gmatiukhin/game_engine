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
    Panels(crate::gfx::texture::Color, Vec<GUIPanel>),
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
                texture::Texture::from_image(device, queue, &img.file, &img.name, false),
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
                let image = surface.image();
                (
                    texture::Texture::from_image(&device, &queue, &image, "Surface image", true),
                    vec![],
                )
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
    default_color: image::Rgba<u8>,
    image: image::RgbaImage,
}

impl Surface2D {
    pub fn new(width: u32, height: u32, default_color: texture::Color) -> Self {
        let default_color = crate::util::from_color_to_rgba(&default_color);
        let mut image: image::RgbaImage = image::ImageBuffer::new(width, height);
        for pixel in image.pixels_mut() {
            *pixel = default_color;
        }

        Self {
            width,
            height,
            default_color,
            image,
        }
    }

    pub fn set_cell_color(&mut self, x: u32, y: u32, color: texture::Color) {
        if x > self.width || y > self.height {
            return;
        }
        self.image.put_pixel(x, y, crate::util::from_color_to_rgba(&color));
    }

    pub fn clear(&mut self) {
        for el in self.image.pixels_mut() {
            *el = self.default_color;
        }
    }

    fn image(&self) -> image::DynamicImage {
        image::DynamicImage::ImageRgba8(self.image.clone())
    }
}
