use skia_safe::{Canvas, Color, Font, Paint, Point, Rect};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct DrawText {
    top: f32,
    left: f32,
    bottom: f32,
    text: String,
    font: Font,
}

impl DrawText {
    pub fn execute(&self, scroll: f32, canvas: &Canvas, paint: &mut Paint) {
        let point = Point::new(self.left, self.top - scroll);
        canvas.draw_str(&self.text, point, &self.font, paint);
    }
}

impl Display for DrawText {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "DrawText(top={} left={} bottom={} text={} font={})",
            self.top,
            self.left,
            self.bottom,
            self.text,
            self.font.typeface().family_name()
        )
    }
}

#[derive(Debug)]
pub struct DrawRect {
    top: f32,
    left: f32,
    bottom: f32,
    right: f32,
    color: Color,
}

impl DrawRect {
    pub fn execute(&self, scroll: f32, canvas: &Canvas, paint: &mut Paint) {
        let rect = Rect::new(
            self.left,
            self.top - scroll,
            self.right,
            self.bottom - scroll,
        );
        paint.set_color(self.color);
        canvas.draw_rect(rect, paint);
    }
}

impl Display for DrawRect {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let rgb = self.color.to_rgb();
        write!(
            f,
            "DrawRect(top={} left={} bottom={} right={} color={}{}{})",
            self.top, self.left, self.bottom, self.right, rgb.r, rgb.g, rgb.b
        )
    }
}

#[derive(Debug)]
pub enum DrawCommand {
    Text(DrawText),
    Rect(DrawRect),
}

impl DrawCommand {
    pub fn text(x1: f32, y1: f32, text: String, font: Font) -> Self {
        let bottom = y1 - font.metrics().1.ascent - font.metrics().1.descent;

        Self::Text(DrawText {
            top: y1,
            left: x1,
            bottom,
            text,
            font,
        })
    }

    pub fn rect(x1: f32, y1: f32, x2: f32, y2: f32, color: Color) -> Self {
        Self::Rect(DrawRect {
            top: y1,
            left: x1,
            bottom: y2,
            right: x2,
            color,
        })
    }

    pub fn execute(&self, scroll: f32, canvas: &Canvas, paint: &mut Paint) {
        match self {
            Self::Text(text) => text.execute(scroll, canvas, paint),
            Self::Rect(rect) => rect.execute(scroll, canvas, paint),
        }
    }

    pub fn get_bottom(&self) -> f32 {
        match self {
            Self::Text(text) => text.bottom,
            Self::Rect(rect) => rect.bottom,
        }
    }

    pub fn get_top(&self) -> f32 {
        match self {
            Self::Text(text) => text.top,
            Self::Rect(rect) => rect.top,
        }
    }
}

impl Display for DrawCommand {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Self::Text(text) => write!(f, "{}", text),
            Self::Rect(rect) => write!(f, "{}", rect),
        }
    }
}
