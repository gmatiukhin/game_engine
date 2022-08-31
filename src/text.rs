use ab_glyph::*;

pub(crate) struct TextRasterizer {
    font: FontRef<'static>,
}

impl TextRasterizer {
    pub(crate) fn new() -> Self {
        let font = FontRef::try_from_slice(include_bytes!("../res/fonts/pixel/Pixel NES.otf"))
                .unwrap();

        Self { font }
    }

    pub(crate) fn get_rasterized_data_from_text(&self, text: &TextParameters, width: u32, height: u32) -> Vec<u8> {
        if let Some(px_scale) = self.font.pt_to_px_scale(text.scale) {
            let scaled_font = self.font.as_scaled(px_scale);
            let glyphs = self.layout_paragraph(&scaled_font, (0.0, 0.0).into(), width, &text.text);
            self.rasterize(&scaled_font, glyphs, width, height, &text.color)
        } else {
            vec![0; width as usize * 4 * height as usize]
        }
    }

    fn layout_paragraph(&self, scaled_font: &PxScaleFont<&FontRef>, start_position: Point, width: u32, text: &str) -> Vec<Glyph> {
        let mut target: Vec<Glyph> = vec![];

        let v_advance = scaled_font.height() + scaled_font.line_gap();
        let max_x_position = start_position.x + width as f32;

        let mut caret = start_position + point(0.0, scaled_font.ascent());
        for c in text.chars() {
            if c.is_control() {
                if c == '\n' {
                    caret = point(start_position.x, caret.y + v_advance);
                }
                continue;
            }
            let mut glyph = scaled_font.scaled_glyph(c);
            glyph.position = caret;
            caret.x += scaled_font.h_advance(glyph.id);

            if !c.is_whitespace() && caret.x > max_x_position {
                caret = point(start_position.x, caret.y + v_advance);
                glyph.position = caret;
                caret.x += scaled_font.h_advance(glyph.id);
            }

            target.push(glyph);
        }

        target
    }

    fn rasterize(&self, scaled_font: &PxScaleFont<&FontRef>, glyphs: Vec<Glyph>, width: u32, height: u32, color: &wgpu::Color) -> Vec<u8> {
        let width = width as usize;
        let height = height as usize;

        let mut data = vec![0; width * 4 * height];

        for glyph in glyphs {
            let position = glyph.position;
            if let Some(outline) = scaled_font.outline_glyph(glyph) {
                outline.draw(|x, y, c| {
                    let y = (position.y as usize + y as usize) * width;
                    let x = position.x as usize + x as usize;

                    let c = c as f64;
                    if (y + x) * 4 + 3 < data.len() {
                        data[(y + x) * 4] = (c * color.r * 255.0) as u8;
                        data[(y + x) * 4 + 1] = (c * color.g * 255.0) as u8;
                        data[(y + x) * 4 + 2] = (c * color.b * 255.0) as u8;
                        data[(y + x) * 4 + 3] = (c * color.a * 255.0) as u8;
                    }
                })
            }
        }

        data
    }
}

pub struct TextParameters {
    pub text: String,
    pub color: wgpu::Color,
    /// Text scale in points
    pub scale: f32,
}
