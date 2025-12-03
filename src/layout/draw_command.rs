use skia_safe::{Canvas, Color, Font, Paint, Point, Rect};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct DrawText {
    top: f32,
    left: f32,
    baseline: f32,
    bottom: f32,
    text: String,
    font: Font,
    color_str: String,
    color: Color,
}

impl DrawText {
    pub fn execute(&self, scroll: f32, canvas: &Canvas, paint: &mut Paint) {
        let point = Point::new(self.left, self.baseline - scroll);
        paint.set_color(self.color);
        canvas.draw_str(&self.text, point, &self.font, paint);
    }
}

impl Display for DrawText {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "DrawText(top={} left={} baseline={} bottom={} font_family={} font_size={} color={} text={})",
            self.top,
            self.left,
            self.baseline,
            self.bottom,
            self.font.typeface().family_name(),
            self.font.size(),
            self.color_str,
            self.text,
        )
    }
}

#[derive(Debug)]
pub struct DrawRect {
    top: f32,
    left: f32,
    bottom: f32,
    right: f32,
    color_str: String,
    color: Option<Color>,
}

impl DrawRect {
    pub fn execute(&self, scroll: f32, canvas: &Canvas, paint: &mut Paint) {
        let rect = Rect::new(
            self.left,
            self.top - scroll,
            self.right,
            self.bottom - scroll,
        );

        if let Some(color) = self.color {
            paint.set_color(color);
        }

        canvas.draw_rect(rect, paint);
    }
}

impl Display for DrawRect {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "DrawRect(top={} left={} bottom={} right={} color={})",
            self.top, self.left, self.bottom, self.right, self.color_str
        )
    }
}

#[derive(Debug)]
pub enum DrawCommand {
    Text(DrawText),
    Rect(DrawRect),
}

impl DrawCommand {
    pub fn text(x1: f32, y1: f32, baseline: f32, text: String, font: Font, color: &str) -> Self {
        let bottom = y1 + font.spacing();

        Self::Text(DrawText {
            top: y1,
            left: x1,
            baseline,
            bottom,
            text,
            font,
            color_str: color.to_string(),
            color: Self::parse_css_color(color).unwrap_or(Color::BLACK),
        })
    }

    fn parse_css_color(color_str: &str) -> Option<Color> {
        match csscolorparser::parse(color_str) {
            Ok(color) => {
                let [r, g, b, a] = color.to_rgba8();
                Some(Color::from_argb(a, r, g, b))
            }
            Err(err) => {
                println!("Error parsing css color: {}", err);
                None
            }
        }
    }

    pub fn rect(x1: f32, y1: f32, x2: f32, y2: f32, color: &str) -> Self {
        Self::Rect(DrawRect {
            top: y1,
            left: x1,
            bottom: y2,
            right: x2,
            color: Self::parse_css_color(color),
            color_str: color.to_string(),
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
