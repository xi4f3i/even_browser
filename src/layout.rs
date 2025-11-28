use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use skia_safe::{
    Font,
    font_style::{Slant, Weight},
};

use crate::{
    constant::{BLOCK_ELEMENTS, HSTEP, VSTEP, WIDTH},
    font::get_font,
    parser::html_node::{HTMLNode, HTMLNodeData},
};

#[derive(Debug)]
pub enum LayoutMode {
    Inline,
    Block,
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutType {
    Document,
    Block,
}

#[derive(Debug)]
pub struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font: Font,
}

#[derive(Debug)]
pub struct LayoutNode {
    pub html_node: Rc<RefCell<HTMLNode>>,
    pub layout_type: LayoutType,
    pub parent: Option<Weak<RefCell<LayoutNode>>>,
    pub previous: Option<Weak<RefCell<LayoutNode>>>,
    pub children: Vec<Rc<RefCell<LayoutNode>>>,

    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,

    cursor_x: f32,
    cursor_y: f32,
    weight: Weight,
    style: Slant,
    size: i32,
    // (x_start, word, font)
    line: Vec<(f32, String, Font)>,
    display_list: Vec<DisplayItem>,
}

impl LayoutNode {
    pub fn new_document(html_node: Rc<RefCell<HTMLNode>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            html_node,
            layout_type: LayoutType::Document,
            parent: None,
            previous: None,
            children: Vec::new(),

            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,

            // display_list: Vec::new(),
            cursor_x: 0.0,
            cursor_y: 0.0,
            weight: Weight::NORMAL,
            style: Slant::Upright,
            size: 12,
            line: Vec::new(),
            display_list: Vec::new(),
        }))
    }

    fn new_block(
        html_node: Rc<RefCell<HTMLNode>>,
        parent: Weak<RefCell<LayoutNode>>,
        previous: Option<Weak<RefCell<LayoutNode>>>,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            html_node,
            layout_type: LayoutType::Block,
            parent: Some(parent),
            previous,
            children: Vec::new(),

            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,

            // display_list: Vec::new(),
            cursor_x: 0.0,
            cursor_y: 0.0,
            weight: Weight::NORMAL,
            style: Slant::Upright,
            size: 12,
            line: Vec::new(),
            display_list: Vec::new(),
        }))
    }

    pub fn layout(self_rc: Rc<RefCell<Self>>) {
        let layout_type = self_rc.borrow().layout_type;
        match layout_type {
            LayoutType::Document => Self::layout_document(self_rc),
            LayoutType::Block => Self::layout_block(self_rc),
        }
    }

    fn layout_document(doc_rc: Rc<RefCell<Self>>) {
        let child = LayoutNode::new_block(
            doc_rc.borrow().html_node.clone(),
            Rc::downgrade(&doc_rc),
            None,
        );

        {
            let mut document = doc_rc.borrow_mut();
            document.children.push(child.clone());

            document.width = WIDTH - 2.0 * HSTEP;
            document.x = HSTEP;
            document.y = VSTEP;
        }

        Self::layout(child.clone());

        doc_rc.borrow_mut().height = child.borrow().height;
    }

    fn layout_block(block_rc: Rc<RefCell<Self>>) {
        {
            let mut block = block_rc.borrow_mut();
            let parent_rc = block
                .parent
                .as_ref()
                .expect("Parent block not found")
                .upgrade()
                .expect("Parent block weak not found");
            let parent = parent_rc.borrow();
            block.x = parent.x;
            block.width = parent.width;
            block.y = parent.y;

            if let Some(previous) = &block.previous {
                let previous_rc = previous.upgrade().expect("Previous block not found");
                let prev = previous_rc.borrow();
                block.y = prev.y + prev.height;
            }
        }

        let mode = block_rc.borrow().layout_mode();

        match mode {
            LayoutMode::Block => {
                let mut prev: Option<Weak<RefCell<Self>>> = None;
                let html_node_rc = block_rc.borrow().html_node.clone();
                let html_node = html_node_rc.borrow();

                for child in html_node.children.iter() {
                    let next = Self::new_block(
                        child.clone(),
                        Rc::downgrade(&block_rc),
                        match prev {
                            Some(p) => Some(p),
                            None => None,
                        },
                    );
                    block_rc.borrow_mut().children.push(next.clone());
                    prev = Some(Rc::downgrade(&next));
                }
            }
            LayoutMode::Inline => {
                let html_node = block_rc.borrow().html_node.clone();

                let mut block = block_rc.borrow_mut();
                block.cursor_x = 0.0;
                block.cursor_y = 0.0;
                block.weight = Weight::NORMAL;
                block.style = Slant::Upright;
                block.size = 12;
                block.line.clear();

                block.recurse(html_node);
                block.flush();
            }
        }

        for child in block_rc.borrow().children.iter() {
            Self::layout(child.clone());
        }

        let mut block = block_rc.borrow_mut();
        match mode {
            LayoutMode::Block => {
                block.height = block
                    .children
                    .iter()
                    .map(|child| child.borrow().height)
                    .sum();
            }
            LayoutMode::Inline => {
                block.height = block.cursor_y;
            }
        }
    }

    fn recurse(&mut self, html_node: Rc<RefCell<HTMLNode>>) {
        let html_node = html_node.borrow();
        match &html_node.data {
            HTMLNodeData::Text(text) => {
                for word in text.text.split_whitespace() {
                    self.word(word);
                }
            }
            HTMLNodeData::Element(e) => {
                self.open_tag(&e.tag);
                for child in html_node.children.iter() {
                    self.recurse(child.clone());
                }
                self.close_tag(&e.tag);
            }
        }
    }

    fn open_tag(&mut self, tag: &str) {
        match tag {
            "i" => self.style = Slant::Italic,
            "b" => self.weight = Weight::BOLD,
            "small" => self.size -= 2,
            "big" => self.size += 4,
            "br" => self.flush(),
            _ => {}
        }
    }

    fn close_tag(&mut self, tag: &str) {
        match tag {
            "i" => self.style = Slant::Upright,
            "b" => self.weight = Weight::NORMAL,
            "small" => self.size += 2,
            "big" => self.size -= 4,
            "p" => {
                self.flush();
                self.cursor_y += VSTEP;
            }
            _ => {}
        }
    }

    fn flush(&mut self) {
        if self.line.is_empty() {
            return;
        }

        let mut max_ascent: f32 = 0.0;
        let mut max_descent: f32 = 0.0;

        for (_, _, font) in &self.line {
            let metrics = font.metrics().1;
            max_ascent = max_ascent.max(metrics.ascent);
            max_descent = max_descent.max(metrics.descent);
        }

        let baseline = self.cursor_y + 1.25 * max_ascent;

        for (real_x, word, font) in self.line.drain(..) {
            let x = self.x + real_x;
            let y = self.y + baseline - font.metrics().1.ascent;
            self.display_list.push(DisplayItem {
                x,
                y,
                text: word,
                font,
            });
        }

        self.cursor_x = 0.0;
        self.cursor_y = baseline + 1.25 * max_descent;
    }

    fn word(&mut self, word: &str) {
        let font = get_font(self.size, self.weight, self.style);
        let w = font.measure_str(word, None).1.width();

        if self.cursor_x + w > self.width {
            self.flush();
        }

        self.line
            .push((self.cursor_x, word.to_string(), font.clone()));

        let space_w = font.measure_str(" ", None).1.width();

        self.cursor_x += w + space_w;
    }

    fn layout_mode(&self) -> LayoutMode {
        let html_node = self.html_node.borrow();
        match &html_node.data {
            HTMLNodeData::Text(_) => LayoutMode::Inline,
            HTMLNodeData::Element(_) => {
                let has_block_child =
                    html_node
                        .children
                        .iter()
                        .any(|child| match &child.borrow().data {
                            HTMLNodeData::Element(e) => BLOCK_ELEMENTS.contains(&e.tag.as_str()),
                            _ => false,
                        });
                if has_block_child {
                    LayoutMode::Block
                } else if !html_node.children.is_empty() {
                    LayoutMode::Inline
                } else {
                    LayoutMode::Block
                }
            }
        }
    }
}
