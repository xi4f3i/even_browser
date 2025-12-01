use skia_safe::font_style::{Slant, Weight, Width};
use skia_safe::{Font, FontMgr, FontStyle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const FAMILY_NAMES: [&str; 3] = ["PingFang SC", "Microsoft YaHei UI", "Noto Sans CJK SC"];

pub type FontManagerRef = Rc<RefCell<FontManager>>;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
struct FontKey(i32, bool, bool); // (font size, is bold, is italic)

impl FontKey {
    fn new(size: i32, weight: Weight, slant: Slant) -> Self {
        let is_bold = weight == Weight::BOLD;
        let is_italic = slant == Slant::Italic;

        Self(size, is_bold, is_italic)
    }
}

#[derive(Debug)]
pub struct FontManager {
    font_cache: HashMap<FontKey, Font>,
    font_mgr: FontMgr,
}

impl FontManager {
    pub fn new() -> FontManagerRef {
        Rc::new(RefCell::new(Self {
            font_cache: HashMap::new(),
            font_mgr: FontMgr::new(),
        }))
    }

    pub fn get_font(&mut self, size: i32, weight: Weight, slant: Slant) -> Font {
        let key = FontKey::new(size, weight, slant);

        if let Some(font) = self.font_cache.get(&key) {
            return font.clone();
        }

        let font_style = FontStyle::new(weight, Width::NORMAL, slant);

        let mut typeface = None;
        for name in FAMILY_NAMES {
            if let Some(tf) = self.font_mgr.match_family_style(name, font_style) {
                typeface = Some(tf);
                break;
            }
        }

        let typeface = typeface.expect("Error: No fonts found at all.");

        let font = Font::new(typeface, size as f32);

        self.font_cache.insert(key, font.clone());

        font
    }
}
