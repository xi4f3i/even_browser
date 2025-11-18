use std::collections::HashMap;

use skia_safe::{
    Font, FontMgr, FontStyle,
    font_style::{Slant, Weight, Width},
};

use crate::{
    constant::{HSTEP, VSTEP, WIDTH},
    lexer::Token,
};

// (大小, 是否粗体, 是否斜体)
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
struct FontKey(i32, bool, bool);

#[derive(Debug)]
pub struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font: Font,
}

#[derive(Debug)]
pub struct Layout {
    font_cache: HashMap<FontKey, Font>,
    font_mgr: FontMgr,

    pub display_list: Vec<DisplayItem>,

    cursor_x: f32,
    cursor_y: f32,

    weight: Weight,
    style: Slant,
    size: i32,

    // (x_start, word, font)
    line: Vec<(f32, String, Font)>,
}

impl Layout {
    pub fn new(tokens: Vec<Token>) -> Self {
        let mut layout = Self {
            font_cache: HashMap::new(),
            font_mgr: FontMgr::new(),
            display_list: Vec::new(),
            cursor_x: HSTEP,
            cursor_y: VSTEP,
            weight: Weight::NORMAL,
            style: Slant::Upright,
            size: 12,
            line: Vec::new(),
        };

        for token in tokens {
            layout.process_token(token);
        }

        layout.flush();

        layout
    }

    fn flush(&mut self) {
        if self.line.is_empty() {
            return;
        }

        let mut max_ascent = 0.0;
        let mut line_metrics = Vec::new();

        for (_, _, font) in &self.line {
            let metrics = font.metrics().1;
            let ascent = -metrics.ascent;
            if ascent > max_ascent {
                max_ascent = ascent;
            }
            line_metrics.push(metrics);
        }

        let baseline = self.cursor_y + 1.25 * max_ascent;

        for ((x, word, font), _metrics) in self.line.drain(..).zip(line_metrics.iter()) {
            let y = baseline;
            self.display_list.push(DisplayItem {
                x,
                y,
                text: word,
                font,
            });
        }

        let mut max_descent = 0.0;
        for metrics in &line_metrics {
            if metrics.descent > max_descent {
                max_descent = metrics.descent;
            }
        }

        self.cursor_y = baseline + 1.25 * max_descent;

        self.cursor_x = HSTEP;
    }

    fn process_token(&mut self, token: Token) {
        match token {
            Token::Text(text) => {
                for word in text.split_whitespace() {
                    self.word(word);
                }
            }
            Token::Tag(tag) => match tag.as_str() {
                "i" => self.style = Slant::Italic,
                "/i" => self.style = Slant::Upright,
                "b" => self.weight = Weight::BOLD,
                "/b" => self.weight = Weight::NORMAL,
                "small" => self.size = self.size.saturating_sub(2),
                "/small" => self.size = self.size.saturating_add(2),
                "big" => self.size = self.size.saturating_add(4),
                "/big" => self.size = self.size.saturating_sub(4),
                "br" => self.flush(),
                "/p" => {
                    self.flush();
                    self.cursor_y += VSTEP;
                }
                _ => {}
            },
        }
    }

    fn word(&mut self, word: &str) {
        let font = self.get_font(self.size, self.weight, self.style);

        let w = font.measure_str(word, None).1.width();

        if self.cursor_x + w >= WIDTH - HSTEP {
            self.flush();
        }

        self.line
            .push((self.cursor_x, word.to_string(), font.clone()));

        let space_w = font.measure_str(" ", None).1.width();
        self.cursor_x += w + space_w;
    }

    fn get_font(&mut self, size: i32, weight: Weight, slant: Slant) -> Font {
        let is_bold = weight == Weight::BOLD;
        let is_italic = slant == Slant::Italic;
        let key = FontKey(size, is_bold, is_italic);

        if let Some(font) = self.font_cache.get(&key) {
            return font.clone();
        }

        let font_style = FontStyle::new(weight, Width::NORMAL, slant);

        let typeface = self
            .font_mgr
            .match_family_style("PingFang SC", font_style)
            .expect("Cannot find PingFang SC font");

        let font = Font::new(typeface, size as f32);

        self.font_cache.insert(key, font.clone());

        font
    }
}
