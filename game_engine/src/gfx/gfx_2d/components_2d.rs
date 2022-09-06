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
    Color(texture::Color),
    Text(super::text::TextParameters),
    Surface2D(Surface2D),
}

pub struct GUIPanel {
    pub name: String,
    pub active: bool,
    /// Position of the top-left corner of the panel
    pub position: GUITransform,
    pub dimensions: GUITransform,

    pub content: GUIPanelContent,
    pub children: Vec<GUIPanel>,
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

        let texture = match &mut self.content {
            GUIPanelContent::Image(img) => {
                texture::Texture::from_image(device, queue, &img.file, &img.name, false)
            }
            GUIPanelContent::Text(text) => {
                let width: u32 = (right - left) as u32;
                let height: u32 = (bottom - top) as u32;
                let data = text_rasterizer.get_rgba_from_text(text, width, height);

                texture::Texture::from_bytes_rgba(device, queue, data, width, height)
            }
            GUIPanelContent::Color(color) => texture::Texture::from_color(device, queue, color),
            GUIPanelContent::Surface2D(surface) => {
                let image = surface.image();
                texture::Texture::from_image(&device, &queue, &image, "Surface image", true)
            }
        };

        let mut buffered_children = vec![];

        for child in &mut self.children {
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
            children: buffered_children,
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

    pub fn draw_point(&mut self, position: cgmath::Point2<u32>, color: texture::Color) {
        if position.x >= self.width || position.y >= self.height {
            return;
        }
        self.image.put_pixel(
            position.x,
            position.y,
            crate::util::from_color_to_rgba(&color),
        );
    }

    /// Draws line from `start` to `end` using [Bresenham's line algorithm](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm) with optimisations
    pub fn draw_line(
        &mut self,
        start: cgmath::Point2<u32>,
        end: cgmath::Point2<u32>,
        color: texture::Color,
    ) {
        let dx = i32::abs(end.x as i32 - start.x as i32);
        let dy = i32::abs(end.y as i32 - start.y as i32);

        // Optimisation for straight vertical line
        if dx == 0 {
            let (y0, y1) = if end.y < start.y {
                (end.y, start.y)
            } else {
                (start.y, end.y)
            };

            for y in y0..=y1 {
                self.draw_point((start.x, y).into(), color);
            }
            return;
        }

        // Optimisation for straight horizontal line
        if dy == 0 {
            let (x0, x1) = if end.x < start.x {
                (end.x, start.x)
            } else {
                (start.x, end.x)
            };

            for x in x0..=x1 {
                self.draw_point((x, start.y).into(), color);
            }
            return;
        }

        // The algorithm itself
        if dy < dx {
            if start.x > end.x {
                self.draw_line_low(end, start, color);
            } else {
                self.draw_line_low(start, end, color);
            }
        } else {
            if start.y > end.y {
                self.draw_line_high(end, start, color);
            } else {
                self.draw_line_high(start, end, color);
            }
        }
    }

    fn draw_line_high(
        &mut self,
        start: cgmath::Point2<u32>,
        end: cgmath::Point2<u32>,
        color: texture::Color,
    ) {
        let mut dx = end.x as i32 - start.x as i32;
        let dy = end.y as i32 - start.y as i32;
        let mut xi = 1;

        if dx < 0 {
            xi = -1;
            dx = -dx;
        }
        let mut d = (2 * dx) - dy;
        let mut x = start.x as i32;

        for y in start.y..=end.y {
            self.draw_point((x as u32, y).into(), color);

            if d > 0 {
                x += xi;
                d += 2 * (dx - dy);
            } else {
                d += 2 * dx;
            }
        }
    }

    fn draw_line_low(
        &mut self,
        start: cgmath::Point2<u32>,
        end: cgmath::Point2<u32>,
        color: texture::Color,
    ) {
        let dx = end.x as i32 - start.x as i32;
        let mut dy = end.y as i32 - start.y as i32;
        let mut yi = 1;

        if dy < 0 {
            yi = -1;
            dy = -dy;
        }

        let mut d = (2 * dy) - dx;
        let mut y = start.y as i32;

        for x in start.x..=end.x {
            self.draw_point((x, y as u32).into(), color);
            if d > 0 {
                y += yi;
                d += 2 * (dy - dx);
            } else {
                d += 2 * dy;
            }
        }
    }

    pub fn draw_rectangle(
        &mut self,
        start: cgmath::Point2<u32>,
        end: cgmath::Point2<u32>,
        color: texture::Color,
        fill: bool,
    ) {
        if fill {
            let (x0, x1) = if end.x < start.x {
                (end.x, start.x)
            } else {
                (start.x, end.x)
            };

            let (y0, y1) = if end.y < start.y {
                (end.y, start.y)
            } else {
                (start.y, end.y)
            };

            for y in y0..=y1 {
                for x in x0..=x1 {
                    self.draw_point((x, y).into(), color);
                }
            }
        } else {
            self.draw_line((start.x, start.y).into(), (end.x, start.y).into(), color);
            self.draw_line((end.x, start.y).into(), (end.x, end.y).into(), color);
            self.draw_line((end.x, end.y).into(), (start.x, end.y).into(), color);
            self.draw_line((start.x, end.y).into(), (start.x, start.y).into(), color);
        }
    }

    /// Draws circle using [modified Bresenham's circle drawing algorithm](https://weber.itn.liu.se/~stegu/circle/circlealgorithm.pdf)
    pub fn draw_circle(
        &mut self,
        center: cgmath::Point2<u32>,
        radius: u32,
        color: texture::Color,
        fill: bool,
    ) {
        let mut x: i32 = 0;
        let mut y: i32 = radius as i32;
        let mut d = 5 - 4 * radius as i32;
        let mut da = 12;
        let mut db = 20 - 8 * radius as i32;

        while x <= y {
            if fill {
                self.draw_circle_octants_filled(center, x, y, color);
            } else {
                self.draw_circle_octants(center, x, y, color);
            }

            if d < 0 {
                d += da;
                db += 8;
            } else {
                y -= 1;
                d += db;
                db += 16;
            }
            x += 1;
            da += 8;
        }
    }

    #[rustfmt::skip]
    fn draw_circle_octants(&mut self, center: cgmath::Point2<u32>, x: i32, y: i32, color: texture::Color) {
        self.draw_point(((center.x as i32 + x) as u32, (center.y as i32 + y) as u32).into(), color);
        self.draw_point(((center.x as i32 - x) as u32, (center.y as i32 + y) as u32).into(), color);
        self.draw_point(((center.x as i32 + x) as u32, (center.y as i32 - y) as u32).into(), color);
        self.draw_point(((center.x as i32 - x) as u32, (center.y as i32 - y) as u32).into(), color);
        self.draw_point(((center.x as i32 + y) as u32, (center.y as i32 + x) as u32).into(), color);
        self.draw_point(((center.x as i32 - y) as u32, (center.y as i32 + x) as u32).into(), color);
        self.draw_point(((center.x as i32 + y) as u32, (center.y as i32 - x) as u32).into(), color);
        self.draw_point(((center.x as i32 - y) as u32, (center.y as i32 - x) as u32).into(), color);
    }

    #[rustfmt::skip]
    fn draw_circle_octants_filled(&mut self, center: cgmath::Point2<u32>, x: i32, y: i32, color: texture::Color) {
        self.draw_line(((center.x as i32 - x) as u32, (center.y as i32 + y) as u32).into(), ((center.x as i32 + x) as u32, (center.y as i32 + y) as u32).into(), color);
        self.draw_line(((center.x as i32 - x) as u32, (center.y as i32 - y) as u32).into(), ((center.x as i32 + x) as u32, (center.y as i32 - y) as u32).into(), color);
        self.draw_line(((center.x as i32 - y) as u32, (center.y as i32 + x) as u32).into(), ((center.x as i32 + y) as u32, (center.y as i32 + x) as u32).into(), color);
        self.draw_line(((center.x as i32 - y) as u32, (center.y as i32 - x) as u32).into(), ((center.x as i32 + y) as u32, (center.y as i32 - x) as u32).into(), color);
    }

    /// Draw triangle using [Standard Algorithm](http://www.sunshine2k.de/coding/java/TriangleRasterization/TriangleRasterization.html#:~:text=II.%20Standard%20Algorithm)
    pub fn draw_triangle(
        &mut self,
        p0: cgmath::Point2<u32>,
        p1: cgmath::Point2<u32>,
        p2: cgmath::Point2<u32>,
        color: texture::Color,
        fill: bool,
    ) {
        if fill {
            self.draw_triangle_filled(p0, p1, p2, color);
        } else {
            self.draw_line(p0, p1, color);
            self.draw_line(p1, p2, color);
            self.draw_line(p2, p0, color);
        }
    }

    /// Draw triangle filled
    fn draw_triangle_filled(
        &mut self,
        p0: cgmath::Point2<u32>,
        p1: cgmath::Point2<u32>,
        p2: cgmath::Point2<u32>,
        color: texture::Color,
    ) {
        // Sort vertices by y-coordinate ascending
        let (p0, p1, p2) = if p0.y > p1.y {
            if p1.y > p2.y {
                (p2, p1, p0)
            } else if p0.y > p2.y {
                (p1, p2, p0)
            } else {
                (p1, p0, p2)
            }
        } else if p0.y > p2.y {
            (p2, p0, p1)
        } else if p1.y > p2.y {
            (p0, p2, p1)
        } else {
            (p0, p1, p2)
        };

        // Check for trivial cases: bottom-flat and top-flat triangles
        if p1.y == p2.y {
            self.draw_triangle_bottom_flat(p0, p1, p2, color);
        } else if p0.y == p1.y {
            self.draw_triangle_top_flat(p0, p1, p2, color);
        } else {
            // General case - split the triangle in a top-flat and bottom-flat one
            let p3 = cgmath::Point2::new(
                (p0.x as f32
                    + ((p1.y as i32 - p0.y as i32) as f32 / (p2.y as i32 - p0.y as i32) as f32
                        * (p2.x as i32 - p0.x as i32) as f32)) as u32,
                p1.y,
            );
            self.draw_triangle_bottom_flat(p0, p1, p3, color);
            self.draw_triangle_top_flat(p1, p3, p2, color);
        }
    }

    // Draw bottom-flat triangle
    fn draw_triangle_bottom_flat(
        &mut self,
        p0: cgmath::Point2<u32>,
        p1: cgmath::Point2<u32>,
        p2: cgmath::Point2<u32>,
        color: texture::Color,
    ) {
        let inv_slope1 = (p1.x as i32 - p0.x as i32) as f32 / (p1.y as i32 - p0.y as i32) as f32;
        let inv_slope2 = (p2.x as i32 - p0.x as i32) as f32 / (p2.y as i32 - p0.y as i32) as f32;

        let mut current_x1 = p0.x as f32;
        let mut current_x2 = p0.x as f32;

        for scanline_y in p0.y..=p1.y {
            self.draw_line(
                cgmath::Point2::new(current_x1 as u32, scanline_y),
                cgmath::Point2::new(current_x2 as u32, scanline_y),
                color,
            );
            current_x1 += inv_slope1;
            current_x2 += inv_slope2;
        }
    }

    // Draw top-flat triangle
    fn draw_triangle_top_flat(
        &mut self,
        p0: cgmath::Point2<u32>,
        p1: cgmath::Point2<u32>,
        p2: cgmath::Point2<u32>,
        color: texture::Color,
    ) {
        let inv_slope1 = (p2.x as i32 - p0.x as i32) as f32 / (p2.y as i32 - p0.y as i32) as f32;
        let inv_slope2 = (p2.x as i32 - p1.x as i32) as f32 / (p2.y as i32 - p1.y as i32) as f32;

        let mut current_x1 = p2.x as f32;
        let mut current_x2 = p2.x as f32;

        for scanline_y in (p0.y..p2.y).rev() {
            self.draw_line(
                cgmath::Point2::new(current_x1 as u32, scanline_y),
                cgmath::Point2::new(current_x2 as u32, scanline_y),
                color,
            );
            current_x1 -= inv_slope1;
            current_x2 -= inv_slope2;
        }
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
