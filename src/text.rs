use ab_glyph::{point, Font, FontRef, Glyph, Point, PxScaleFont, ScaleFont};

pub fn layout_paragraph<F, SF>(
    font: SF,
    start_position: Point,
    max_width: f32,
    text: &str,
) -> Vec<Glyph>
where
    F: Font,
    SF: ScaleFont<F>,
{
    let mut target: Vec<Glyph> = vec![];
    let v_advance = font.height() + font.line_gap();
    let mut caret = start_position + point(0.0, font.ascent());
    for c in text.chars() {
        if c.is_control() {
            if c == '\n' {
                caret = point(start_position.x, caret.y + v_advance);
            }
            continue;
        }
        let mut glyph = font.scaled_glyph(c);

        if !c.is_whitespace() && caret.x > start_position.x + max_width {
            caret = point(start_position.x, caret.y + v_advance);
        }
        glyph.position = caret;
        caret.x += font.h_advance(glyph.id);

        target.push(glyph);
    }

    target
}

pub fn parse(font: &PxScaleFont<&FontRef>, glyphs: Vec<Glyph>) -> (Vec<f32>, u32, u32) {
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

    let max_x = f32::ceil(max_x + font.scale.x) as usize;
    let max_y = f32::ceil(max_y + font.scale.y) as usize;

    let mut data = vec![0.0; max_x * max_y];

    for glyph in glyphs {
        let position = glyph.position;
        if let Some(outline) = font.outline_glyph(glyph) {
            outline.draw(|x, y, c| {
                data[((position.y as usize + y as usize) * max_x)
                    + (position.x as usize + x as usize)] = c;
            })
        }
    }

    (data, max_x as u32, max_y as u32)
}
