use std::{cell::RefCell, collections::HashMap, rc::Rc};

use skia_safe::{
    Canvas, Font, FontMgr, FontStyle, Paint, Point, Rect,
    font_style::{Slant, Weight, Width},
};

use crate::{constant::BLOCK_ELEMENTS, parser::Node};
use crate::{
    constant::{HSTEP, VSTEP, WIDTH},
    parser::NodeRef,
};

pub type DrawCommandRef = Box<dyn DrawCommand>;

pub trait DrawCommand {
    fn execute(&self, scroll: f32, canvas: &Canvas, paint: &Paint);
    fn get_top(&self) -> f32;
    fn get_bottom(&self) -> f32;
}

pub struct DrawText {
    top: f32,
    left: f32,
    text: String,
    font: Font,
    bottom: f32,
}

impl DrawText {
    pub fn new(x1: f32, y1: f32, text: String, font: Font) -> Self {
        Self {
            top: y1,
            left: x1,
            text,
            font: font.clone(),
            bottom: y1 + font.metrics().1.ascent - font.metrics().1.descent,
        }
    }
}

impl DrawCommand for DrawText {
    fn execute(&self, scroll: f32, canvas: &Canvas, paint: &Paint) {
        let point = Point::new(self.left, self.top - scroll);
        canvas.draw_str(&self.text, point, &self.font, paint);
    }

    fn get_top(&self) -> f32 {
        self.top
    }

    fn get_bottom(&self) -> f32 {
        self.bottom
    }
}

pub struct DrawRect {
    top: f32,
    left: f32,
    bottom: f32,
    right: f32,
    color: String,
}

impl DrawRect {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, color: String) -> Self {
        Self {
            top: y1,
            left: x1,
            bottom: y2,
            right: x2,
            color,
        }
    }
}

impl DrawCommand for DrawRect {
    fn execute(&self, scroll: f32, canvas: &Canvas, paint: &Paint) {
        let rect = Rect::new(
            self.left,
            self.top - scroll,
            self.right,
            self.bottom - scroll,
        );
        canvas.draw_rect(rect, paint);
    }

    fn get_top(&self) -> f32 {
        self.top
    }

    fn get_bottom(&self) -> f32 {
        self.bottom
    }
}

// (size, weight, slant)
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
struct FontKey(i32, bool, bool);

#[derive(Debug)]
pub struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font: Font,
}

pub type LayoutRef = Rc<RefCell<dyn Layout>>;

#[derive(Debug)]
pub struct LayoutData {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

pub trait Layout {
    fn layout(&mut self, self_ref: LayoutRef);

    fn paint(&self) -> Vec<DrawCommandRef>;

    fn get_data(&self) -> &LayoutData;

    fn get_children(&self) -> &Vec<LayoutRef>;

    fn get_data_mut(&mut self) -> &mut LayoutData;
}

pub struct DocumentLayout {
    data: LayoutData,
    node: NodeRef,
    parent: Option<LayoutRef>,
    previous: Option<LayoutRef>,
    children: Vec<LayoutRef>,
}

impl DocumentLayout {
    pub fn new(node: NodeRef) -> Self {
        Self {
            data: LayoutData {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            node,
            parent: None,
            previous: None,
            children: Vec::new(),
        }
    }
}

impl Layout for DocumentLayout {
    fn layout(&mut self, self_ref: LayoutRef) {
        let child = BlockLayout::new(self.node.clone(), Some(self_ref.clone()), None);
        let child_ref = Rc::new(RefCell::new(child));
        self.children.push(child_ref.clone());
        self.data.width = WIDTH - 2.0 * HSTEP;
        self.data.x = HSTEP;
        self.data.y = VSTEP;
        let mut child_borrow = child_ref.borrow_mut();
        child_borrow.data.x = self.data.x;
        child_borrow.data.width = self.data.width;
        child_borrow.data.y = self.data.y;
        child_borrow.layout(child_ref.clone());
        self.data.height = child_borrow.data.height;
    }

    fn paint(&self) -> Vec<DrawCommandRef> {
        Vec::new()
    }

    fn get_data(&self) -> &LayoutData {
        &self.data
    }

    fn get_children(&self) -> &Vec<LayoutRef> {
        &self.children
    }

    fn get_data_mut(&mut self) -> &mut LayoutData {
        &mut self.data
    }
}

enum LayoutMode {
    Inline,
    Block,
}

pub struct BlockLayout {
    data: LayoutData,
    node: NodeRef,
    parent: Option<LayoutRef>,
    previous: Option<LayoutRef>,
    children: Vec<LayoutRef>,
    display_list: Vec<DisplayItem>,
    cursor_x: f32,
    cursor_y: f32,
    weight: Weight,
    style: Slant,
    size: i32,

    // (x_start, word, font)
    line: Vec<(f32, String, Font)>,
    font_cache: HashMap<FontKey, Font>,
    font_mgr: FontMgr,
}

impl BlockLayout {
    pub fn new(node: NodeRef, parent: Option<LayoutRef>, previous: Option<LayoutRef>) -> Self {
        Self {
            data: LayoutData {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            node,
            parent,
            previous,
            children: Vec::new(),
            display_list: Vec::new(),
            cursor_x: 0.0,
            cursor_y: 0.0,
            weight: Weight::NORMAL,
            style: Slant::Upright,
            size: 12,
            line: Vec::new(),
            font_cache: HashMap::new(),
            font_mgr: FontMgr::new(),
        }
    }

    fn layout_mode(&self) -> LayoutMode {
        let node = &*self.node.borrow();
        match node {
            Node::Text(_) => LayoutMode::Inline,
            Node::Element(element) => {
                if element.children.iter().any(|child_ref| {
                    if let Node::Element(child_element) = &*child_ref.borrow() {
                        BLOCK_ELEMENTS.contains(&child_element.tag.as_str())
                    } else {
                        false
                    }
                }) {
                    return LayoutMode::Block;
                } else if !element.children.is_empty() {
                    return LayoutMode::Inline;
                }
                LayoutMode::Block
            }
        }
    }

    fn recurse(&mut self, tree: &Node) {
        match tree {
            Node::Text(text_node) => {
                for word in text_node.text.split_whitespace() {
                    self.word(word);
                }
            }
            Node::Element(elem) => {
                self.open_tag(elem.tag.as_str());
                for child in elem.children.iter() {
                    self.recurse(&*child.borrow());
                }
                self.close_tag(elem.tag.as_str());
            }
        }
    }

    fn word(&mut self, word: &str) {
        let font = self.get_font(self.size, self.weight, self.style);

        let w = font.measure_str(word, None).1.width();
        let space_w = font.measure_str(" ", None).1.width();

        if self.cursor_x + w >= self.data.width {
            self.flush();
        }

        self.line.push((self.cursor_x, word.to_string(), font));

        self.cursor_x += w + space_w;
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

        for (real_x, word, font) in self.line.iter() {
            self.display_list.push(DisplayItem {
                x: self.data.x + real_x,
                y: self.data.y + baseline - font.metrics().1.ascent,
                text: word.clone(),
                font: font.clone(),
            });
        }

        self.cursor_x = 0.0;
        self.line.clear();

        let mut max_descent = 0.0;
        for metrics in &line_metrics {
            if metrics.descent > max_descent {
                max_descent = metrics.descent;
            }
        }

        self.cursor_y = baseline + 1.25 * max_descent;
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

    fn close_tag(&mut self, tag: &str) {
        match tag {
            "i" => self.style = Slant::Upright,
            "b" => self.weight = Weight::NORMAL,
            "small" => self.size = self.size.saturating_add(2),
            "big" => self.size = self.size.saturating_sub(4),
            "p" => {
                self.flush();
                self.cursor_y += VSTEP;
            }
            _ => {}
        }
    }

    fn open_tag(&mut self, tag: &str) {
        match tag {
            "i" => self.style = Slant::Italic,
            "b" => self.weight = Weight::BOLD,
            "small" => self.size = self.size.saturating_sub(2),
            "big" => self.size = self.size.saturating_add(4),
            "br" => self.flush(),
            _ => {}
        }
    }
}

impl Layout for BlockLayout {
    fn get_children(&self) -> &Vec<LayoutRef> {
        &self.children
    }

    fn get_data_mut(&mut self) -> &mut LayoutData {
        &mut self.data
    }

    fn layout(&mut self, self_ref: LayoutRef) {
        // Parent data is now pushed by the caller, so we don't need to borrow parent here.
        // if let Some(parent_ref) = &self.parent {
        //     let parent = &*parent_ref.borrow();
        //     let parent_data = parent.get_data();
        //     self.data.x = parent_data.x;
        //     self.data.width = parent_data.width;
        //     self.data.y = parent_data.y;
        // }

        if let Some(previous_ref) = &self.previous {
            let previous = &*previous_ref.borrow();
            let previous_data = previous.get_data();
            self.data.y = previous_data.y + previous_data.height;
        }

        let mode = self.layout_mode();

        match mode {
            LayoutMode::Block => {
                let mut previous: Option<LayoutRef> = None;

                if let Node::Element(node) = &*self.node.borrow() {
                    for child in node.children.iter() {
                        let next =
                            BlockLayout::new(child.clone(), Some(self_ref.clone()), previous);
                        let next_ref = Rc::new(RefCell::new(next));
                        self.children.push(next_ref.clone());
                        previous = Some(next_ref);
                    }
                }
            }
            LayoutMode::Inline => {
                self.cursor_x = 0.0;
                self.cursor_y = 0.0;
                self.weight = Weight::NORMAL;
                self.style = Slant::Upright;
                self.size = 12;

                self.line.clear();
                let node = self.node.clone();
                self.recurse(&*node.borrow());
                self.flush();
            }
        }

        for child_ref in self.children.iter() {
            let child = &mut *child_ref.borrow_mut();
            {
                let child_data = child.get_data_mut();
                child_data.x = self.data.x;
                child_data.width = self.data.width;
                child_data.y = self.data.y;
            }
            child.layout(child_ref.clone());
        }

        match mode {
            LayoutMode::Block => {
                self.data.height = self
                    .children
                    .iter()
                    .map(|child_ref| child_ref.borrow().get_data().height)
                    .sum();
            }
            LayoutMode::Inline => {
                self.data.height = self.cursor_y;
            }
        }
    }

    fn paint(&self) -> Vec<DrawCommandRef> {
        let mut cmds: Vec<DrawCommandRef> = Vec::new();

        if let Node::Element(e) = &*self.node.borrow() {
            if e.tag == "pre" {
                let x2 = self.data.x + self.data.width;
                let y2 = self.data.y + self.data.height;
                let rect = DrawRect::new(self.data.x, self.data.y, x2, y2, "gray".to_string());
                cmds.push(Box::new(rect));
            }
        }

        if let LayoutMode::Inline = self.layout_mode() {
            for item in self.display_list.iter() {
                cmds.push(Box::new(DrawText::new(
                    item.x,
                    item.y,
                    item.text.clone(),
                    item.font.clone(),
                )));
            }
        }

        cmds
    }

    fn get_data(&self) -> &LayoutData {
        &self.data
    }
}
