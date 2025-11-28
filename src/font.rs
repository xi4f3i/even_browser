use skia_safe::{
    Font, FontMgr, FontStyle,
    font_style::{Slant, Weight, Width},
};
use std::cell::RefCell;
use std::collections::HashMap;

// (size, weight, slant)
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
struct FontKey(i32, bool, bool);

thread_local! {
    static FONTS: RefCell<HashMap<FontKey, Font>> = RefCell::new(HashMap::new());
}

pub fn get_font(size: i32, weight: Weight, slant: Slant) -> Font {
    let is_bold = weight == Weight::BOLD;
    let is_italic = slant == Slant::Italic;
    let key = FontKey(size, is_bold, is_italic);

    FONTS.with(|fonts| {
        let mut fonts = fonts.borrow_mut();
        if let Some(font) = fonts.get(&key) {
            return font.clone();
        }

        let font_style = FontStyle::new(weight, Width::NORMAL, slant);

        let font_mgr = FontMgr::default();

        let typeface = font_mgr
            .match_family_style("PingFang SC", font_style)
            .unwrap_or(
                font_mgr
                    .match_family_style("Microsoft YaHei UI", font_style)
                    .expect("Cannot find PingFang SC font"),
            );

        let font = Font::new(typeface, size as f32);

        fonts.insert(key, font.clone());

        font
    })
}
