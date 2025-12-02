use std::collections::HashMap;
use std::sync::OnceLock;

pub const STYLE_KEY_BACKGROUND_COLOR: &str = "background-color";
pub const BACKGROUND_COLOR_DEFAULT_VALUE: &str = "transparent";
pub const UNIT_PIXEL: &str = "px";
pub const STYLE_KEY_FONT_SIZE: &str = "font-size";
pub const STYLE_KEY_COLOR: &str = "color";

pub const DEFAULT_COLOR_STR: &str = "black";
pub const DEFAULT_FONT_SIZE_NUM: i32 = 12;
pub const DEFAULT_FONT_SIZE: &str = "12px";

static INHERITED_PROPERTIES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

pub fn get_inherited_properties() -> &'static HashMap<&'static str, &'static str> {
    INHERITED_PROPERTIES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("font-size", DEFAULT_FONT_SIZE);
        m.insert("font-style", "normal");
        m.insert("font-weight", "normal");
        m.insert("color", "black");
        m
    })
}
