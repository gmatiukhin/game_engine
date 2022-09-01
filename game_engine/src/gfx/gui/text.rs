use ab_glyph::*;

pub(crate) struct TextRasterizer {
    default_font: FontRef<'static>,
}

impl TextRasterizer {
    pub(crate) fn new() -> Self {
        let default_font =
            FontRef::try_from_slice(include_bytes!("../../../res/fonts/HoneyRoom.ttf")).unwrap();

        Self { default_font }
    }

    pub(crate) fn get_rasterized_data_from_text(
        &self,
        text: &TextParameters,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        if let FontParameters::Custom(data) = text.font {
            if let Ok(font) = FontRef::try_from_slice(data) {
                if let Some(px_scale) = self.default_font.pt_to_px_scale(text.scale) {
                    let scaled_font = font.as_scaled(px_scale);
                    return Self::get_data(&scaled_font, text, width, height);
                }
            }
        } else {
            if let Some(px_scale) = self.default_font.pt_to_px_scale(text.scale) {
                let scaled_font = self.default_font.as_scaled(px_scale);
                return Self::get_data(&scaled_font, text, width, height);
            }
        }

        vec![0; width as usize * 4 * height as usize]
    }

    fn get_data(
        scaled_font: &PxScaleFont<&FontRef>,
        text: &TextParameters,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let glyphs = Self::layout_paragraph(&scaled_font, (0.0, 0.0).into(), width, &text.text);
        Self::rasterize(&scaled_font, glyphs, width, height, &text.color)
    }

    fn layout_paragraph(
        scaled_font: &PxScaleFont<&FontRef>,
        start_position: Point,
        width: u32,
        text: &str,
    ) -> Vec<Glyph> {
        let mut target: Vec<Glyph> = vec![];

        let v_advance = scaled_font.height() + scaled_font.line_gap();
        let max_x_position = start_position.x + width as f32;

        let mut caret = start_position + point(0.0, scaled_font.height());
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

    fn rasterize(
        scaled_font: &PxScaleFont<&FontRef>,
        glyphs: Vec<Glyph>,
        width: u32,
        height: u32,
        color: &wgpu::Color,
    ) -> Vec<u8> {
        let width = width as usize;
        let height = height as usize;

        let mut data = vec![0; width * 4 * height];

        for glyph in glyphs {
            if let Some(outline) = scaled_font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|x, y, c| {
                    let y = bounds.min.y as usize + y as usize;
                    let x = bounds.min.x as usize + x as usize;
                    let index = (y * width + x) * 4;
                    if index + 3 < data.len() {
                        data[index] = (c * color.r as f32 * 255.0) as u8;
                        data[index + 1] = (c * color.g as f32 * 255.0) as u8;
                        data[index + 2] = (c * color.b as f32 * 255.0) as u8;
                        data[index + 3] = (c * color.a as f32 * 255.0) as u8;
                    }
                })
            }
        }

        data
    }
}

pub enum FontParameters {
    Default,
    Custom(&'static [u8]),
}

pub struct TextParameters {
    pub text: String,
    pub color: wgpu::Color,
    /// Text scale in points
    pub scale: f32,
    pub font: FontParameters,
}
