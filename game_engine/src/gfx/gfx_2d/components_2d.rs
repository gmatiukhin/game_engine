use crate::gfx::texture;

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
    pub fn new(width: u32, height: u32, default_color: texture::PixelColor) -> Self {
        let default_color = default_color.into();
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

    pub fn from_image(image: image::RgbaImage) -> Self {
        let (width, height) = image.dimensions();

        Self {
            width,
            height,
            default_color: image::Rgba([0, 0, 0, 0]),
            image,
        }
    }

    pub fn from_color(color: texture::PixelColor) -> Self {
        let default_color = color.into();
        let mut image: image::RgbaImage = image::ImageBuffer::new(1, 1);
        for pixel in image.pixels_mut() {
            *pixel = default_color;
        }

        Self {
            width: 1,
            height: 1,
            default_color,
            image,
        }
    }

    /// Draws a point on the surface
    pub fn draw_color_point(&mut self, position: cgmath::Point2<i32>, color: texture::PixelColor) {
        if position.x as u32 >= self.width || position.y as u32 >= self.height {
            return;
        }
        self.image
            .put_pixel(position.x as u32, position.y as u32, color.into());
    }

    /// Draws a point on the surface with premultiplied alpha blending
    pub fn draw_rgba_point(&mut self, position: cgmath::Point2<i32>, rgba: image::Rgba<u8>) {
        if position.x as u32 >= self.width || position.y as u32 >= self.height {
            return;
        }

        // Premultiplied alpha blending
        let pixel = self
            .image
            .get_pixel_mut(position.x as u32, position.y as u32);
        let alpha = rgba.0[3] as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;
        pixel.0[0] = (pixel.0[0] as f32 * inv_alpha + rgba.0[0] as f32) as u8;
        pixel.0[1] = (pixel.0[1] as f32 * inv_alpha + rgba.0[1] as f32) as u8;
        pixel.0[2] = (pixel.0[2] as f32 * inv_alpha + rgba.0[2] as f32) as u8;
        pixel.0[3] = (pixel.0[3] as f32 * inv_alpha + rgba.0[3] as f32) as u8;
    }

    /// Draws line from `start` to `end` using [Bresenham's line algorithm](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm) with optimisations
    pub fn draw_line(
        &mut self,
        start: cgmath::Point2<i32>,
        end: cgmath::Point2<i32>,
        color: texture::PixelColor,
    ) {
        let dx = i32::abs(end.x - start.x);
        let dy = i32::abs(end.y - start.y);

        // Optimisation for straight vertical line
        if dx == 0 {
            let (y0, y1) = if end.y < start.y {
                (end.y, start.y)
            } else {
                (start.y, end.y)
            };

            for y in y0..=y1 {
                self.draw_color_point((start.x, y).into(), color);
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
                self.draw_color_point((x, start.y).into(), color);
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
        start: cgmath::Point2<i32>,
        end: cgmath::Point2<i32>,
        color: texture::PixelColor,
    ) {
        let mut dx = end.x - start.x;
        let dy = end.y - start.y;
        let mut xi = 1;

        if dx < 0 {
            xi = -1;
            dx = -dx;
        }
        let mut d = (2 * dx) - dy;
        let mut x = start.x;

        for y in start.y..=end.y {
            self.draw_color_point((x, y).into(), color);

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
        start: cgmath::Point2<i32>,
        end: cgmath::Point2<i32>,
        color: texture::PixelColor,
    ) {
        let dx = end.x - start.x;
        let mut dy = end.y - start.y;
        let mut yi = 1;

        if dy < 0 {
            yi = -1;
            dy = -dy;
        }

        let mut d = (2 * dy) - dx;
        let mut y = start.y;

        for x in start.x..=end.x {
            self.draw_color_point((x, y).into(), color);
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
        start: cgmath::Point2<i32>,
        end: cgmath::Point2<i32>,
        color: texture::PixelColor,
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
                    self.draw_color_point((x, y).into(), color);
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
        center: cgmath::Point2<i32>,
        radius: u32,
        color: texture::PixelColor,
        fill: bool,
    ) {
        let mut x = 0;
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
    fn draw_circle_octants(&mut self, center: cgmath::Point2<i32>, x: i32, y: i32, color: texture::PixelColor) {
        self.draw_color_point((center.x + x, center.y + y).into(), color);
        self.draw_color_point((center.x - x, center.y + y).into(), color);
        self.draw_color_point((center.x + x, center.y - y).into(), color);
        self.draw_color_point((center.x - x, center.y - y).into(), color);
        self.draw_color_point((center.x + y, center.y + x).into(), color);
        self.draw_color_point((center.x - y, center.y + x).into(), color);
        self.draw_color_point((center.x + y, center.y - x).into(), color);
        self.draw_color_point((center.x - y, center.y - x).into(), color);
    }

    #[rustfmt::skip]
    fn draw_circle_octants_filled(&mut self, center: cgmath::Point2<i32>, x: i32, y: i32, color: texture::PixelColor) {
        self.draw_line((center.x - x, center.y + y).into(), (center.x + x, center.y + y).into(), color);
        self.draw_line((center.x - x, center.y - y).into(), (center.x + x, center.y - y).into(), color);
        self.draw_line((center.x - y, center.y + x).into(), (center.x + y, center.y + x).into(), color);
        self.draw_line((center.x - y, center.y - x).into(), (center.x + y, center.y - x).into(), color);
    }

    /// Draw triangle using [Standard Algorithm](http://www.sunshine2k.de/coding/java/TriangleRasterization/TriangleRasterization.html#:~:text=II.%20Standard%20Algorithm)
    pub fn draw_triangle(
        &mut self,
        p0: cgmath::Point2<i32>,
        p1: cgmath::Point2<i32>,
        p2: cgmath::Point2<i32>,
        color: texture::PixelColor,
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

    fn draw_triangle_filled(
        &mut self,
        p0: cgmath::Point2<i32>,
        p1: cgmath::Point2<i32>,
        p2: cgmath::Point2<i32>,
        color: texture::PixelColor,
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
            // Floating point calculation is required here, because not every triangle configuration can be correctly split using integer division
            let p3 = cgmath::Point2::new(
                (p0.x as f32
                    + ((p1.y as f32 - p0.y as f32) / (p2.y as f32 - p0.y as f32)
                        * (p2.x as f32 - p0.x as f32))) as i32,
                p1.y,
            );
            self.draw_triangle_bottom_flat(p0, p1, p3, color);
            self.draw_triangle_top_flat(p1, p3, p2, color);
        }
    }

    fn draw_triangle_bottom_flat(
        &mut self,
        p0: cgmath::Point2<i32>,
        p1: cgmath::Point2<i32>,
        p2: cgmath::Point2<i32>,
        color: texture::PixelColor,
    ) {
        let inv_slope1 = (p1.x - p0.x) as f32 / (p1.y - p0.y) as f32;
        let inv_slope2 = (p2.x - p0.x) as f32 / (p2.y - p0.y) as f32;

        let mut current_x1 = p0.x as f32;
        let mut current_x2 = p0.x as f32;

        for scanline_y in p0.y..=p1.y {
            self.draw_line(
                cgmath::Point2::new(current_x1 as i32, scanline_y),
                cgmath::Point2::new(current_x2 as i32, scanline_y),
                color,
            );
            current_x1 += inv_slope1;
            current_x2 += inv_slope2;
        }
    }

    fn draw_triangle_top_flat(
        &mut self,
        p0: cgmath::Point2<i32>,
        p1: cgmath::Point2<i32>,
        p2: cgmath::Point2<i32>,
        color: texture::PixelColor,
    ) {
        let inv_slope1 = (p2.x - p0.x) as f32 / (p2.y - p0.y) as f32;
        let inv_slope2 = (p2.x - p1.x) as f32 / (p2.y - p1.y) as f32;

        let mut current_x1 = p2.x as f32;
        let mut current_x2 = p2.x as f32;

        for scanline_y in (p0.y..p2.y).rev() {
            self.draw_line(
                cgmath::Point2::new(current_x1 as i32, scanline_y),
                cgmath::Point2::new(current_x2 as i32, scanline_y),
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

    /// Draws sprite given its top left corner as position
    pub fn draw_sprite(&mut self, sprite: &image::RgbaImage, position: cgmath::Point2<i32>) {
        for (x, y, pixel) in sprite.enumerate_pixels() {
            let x = x as i32 + position.x;
            let y = y as i32 + position.y;
            self.draw_rgba_point((x, y).into(), *pixel);
        }
    }

    pub fn image(&self) -> image::DynamicImage {
        image::DynamicImage::ImageRgba8(self.image.clone())
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }
}
