use crate::constant::{HSTEP, VSTEP, WIDTH};

#[derive(Debug)]
pub struct Glyph {
    pub x: f32,
    pub y: f32,
    pub c: char,
}

pub fn layout(text: &str) -> Vec<Glyph> {
    let mut display_list = Vec::with_capacity(text.len());
    let mut cursor_x = HSTEP;
    let mut cursor_y = VSTEP;

    for c in text.chars() {
        display_list.push(Glyph {
            x: cursor_x,
            y: cursor_y,
            c,
        });

        cursor_x += HSTEP;

        if cursor_x >= WIDTH - HSTEP {
            cursor_y += VSTEP;
            cursor_x = HSTEP;
        }
    }

    display_list
}
