use crate::parser::css_parser::{CSSParser, CSSRules, PERCENT};
use crate::parser::html_node::{HTMLNodeData, HTMLNodeRef, HTMLNodeWeakRef};
use std::collections::HashMap;
use std::sync::OnceLock;

const DEFAULT_FONT_SIZE: &str = "16px";

// 定义一个全局静态变量，初始为空
static INHERITED_PROPERTIES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

// 获取该 HashMap 的函数
fn get_inherited_properties() -> &'static HashMap<&'static str, &'static str> {
    INHERITED_PROPERTIES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("font-size", DEFAULT_FONT_SIZE);
        m.insert("font-style", "normal");
        m.insert("font-weight", "normal");
        m.insert("color", "black");
        m
    })
}

const ATTRIBUTE_KEY_STYLE: &str = "style";
pub const STYLE_KEY_BACKGROUND_COLOR: &str = "background-color";
pub const BACKGROUND_COLOR_DEFAULT_VALUE: &str = "transparent";

pub fn style(node_rc: HTMLNodeRef, rules: &CSSRules) {
    let node = &mut *node_rc.borrow_mut();
    node.style.clear();

    // Inherited style
    if let Some(parent_weak) = &node.parent
        && let Some(parent_rc) = parent_weak.upgrade()
    {
        for (property, default_value) in get_inherited_properties().iter() {
            if let Some(value) = parent_rc.borrow().style.get(*property) {
                node.style.insert(property.to_string(), value.to_string());
            } else {
                node.style
                    .insert(property.to_string(), default_value.to_string());
            }
        }
    }

    // External style
    for (selector, body) in rules.iter() {
        if !selector.matches(node_rc.clone()) {
            continue;
        }

        for (property, value) in body.iter() {
            node.style.insert(property.to_string(), value.to_string());
        }
    }

    // Inline style
    if let HTMLNodeData::Element(e) = &node.data
        && let Some(style) = e.attributes.get(ATTRIBUTE_KEY_STYLE)
    {
        let pairs = CSSParser::new(style).body();
        node.style.extend(pairs);
    }

    // Calculate percentage font size
    let current_font_size = node.style.get("font-size").cloned();
    if let Some(current_val) = current_font_size
        && current_val.ends_with(PERCENT)
    {
        let parent_font_size = get_parent_font_size(node.parent.clone());

        let node_pct = current_val
            .trim_end_matches(PERCENT)
            .parse::<f32>()
            .unwrap_or(0.0)
            / 100.0;

        let parent_px = parent_font_size
            .trim_end_matches("px")
            .parse::<f32>()
            .unwrap_or(16.0);

        let new_size = node_pct * parent_px;

        node.style
            .insert("font-size".to_string(), format!("{}px", new_size));
    }

    for child in &node.children {
        style(child.clone(), rules);
    }
}

fn get_parent_font_size(parent_weak: Option<HTMLNodeWeakRef>) -> String {
    let Some(parent_weak) = parent_weak else {
        return DEFAULT_FONT_SIZE.to_string();
    };

    let Some(parent_rc) = parent_weak.upgrade() else {
        return DEFAULT_FONT_SIZE.to_string();
    };

    if let Some(font_size) = parent_rc.borrow().style.get("font-size") {
        return font_size.to_string();
    }

    DEFAULT_FONT_SIZE.to_string()
}
