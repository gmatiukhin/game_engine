use ab_glyph::{point, Font, FontRef, Glyph, Point, ScaleFont};

pub fn layout_paragraph<F, SF>(
    font: SF,
    position: Point,
    max_width: f32,
    text: &str,
) -> Vec<Glyph>
    where
    F: Font,
    SF: ScaleFont<F>,
{
    let mut target: Vec<Glyph> = vec![];
    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(0.0, font.ascent());
    let mut last_glyph: Option<Glyph> = None;
    for c in text.chars() {
        if c.is_control() {
            if c == '\n' {
                caret = point(position.x, caret.y + v_advance);
                last_glyph = None;
            }
            continue;
        }
        let mut glyph = font.scaled_glyph(c);
        if let Some(previous) = last_glyph.take() {
            caret.x += font.kern(previous.id, glyph.id);
        }
        glyph.position = caret;

        last_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        if !c.is_whitespace() && caret.x > position.x + max_width {
            caret = point(position.x, caret.y + v_advance);
            glyph.position = caret;
            last_glyph = None;
        }

        target.push(glyph);
    }

    target
}

pub fn parse(font: &impl Font, glyphs: Vec<Glyph>, scale: f32) -> (Vec<f32>, u32, u32) {
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for glyph in &glyphs {
        if glyph.position.x > max_x {
            max_x = glyph.position.x;
        }
        if glyph.position.y > max_y {
            max_y = glyph.position.y;
        }
    }

    let max_x = f32::ceil(max_x + scale) as usize;
    let max_y = f32::ceil(max_y + scale) as usize;

    let mut data = vec![0.0; max_x * max_y];

    for glyph in glyphs {
        let position = glyph.position;
        if let Some(outline) = font.outline_glyph(glyph) {
            outline.draw(|x, y, c| {
                data[((position.y as usize + y as usize) * max_x) + (position.x as usize + x as usize)] = c;
            })
        }
    }

    (data, max_x as u32, max_y as u32)
}
