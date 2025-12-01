use crate::constant::{
    DEFAULT_BROWSER_PADDING, DEFAULT_FONT_SIZE, DEFAULT_X, DEFAULT_Y, VSTEP, WIDTH,
};
use crate::layout::draw_command::DrawCommand;
use crate::layout::font_manager::FontManagerRef;
use crate::layout::layout_mode::LayoutMode;
use crate::parser::html_node::{HTMLNode, HTMLNodeData, HTMLNodeRef};
use skia_safe::font_style::{Slant, Weight};
use skia_safe::{Color, Font};
use std::cell::RefCell;
use std::fmt::{Display, Formatter, Result};
use std::rc::{Rc, Weak};

#[derive(Debug)]
pub struct DisplayItem {
    pub x: f32,
    pub y: f32,
    pub baseline: f32,
    pub text: String,
    pub font: Font,
}

pub type BlockLayoutRef = Rc<RefCell<BlockLayout>>;
pub type BlockLayoutWeakRef = Weak<RefCell<BlockLayout>>;

#[derive(Debug)]
pub struct BlockLayout {
    node: Rc<RefCell<HTMLNode>>,
    parent: Option<BlockLayoutWeakRef>,
    previous: Option<BlockLayoutWeakRef>,
    pub children: Vec<BlockLayoutRef>,
    font_manager: FontManagerRef,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    mode: LayoutMode,
    cursor_x: f32,
    cursor_y: f32,
    weight: Weight,
    style: Slant,
    size: i32,
    // (x_start, word, font)
    line: Vec<(f32, String, Font)>,
    display_list: Vec<DisplayItem>,
}

impl BlockLayout {
    pub fn new(
        node: HTMLNodeRef,
        parent: Option<BlockLayoutWeakRef>,
        previous: Option<BlockLayoutWeakRef>,
        font_manager: FontManagerRef,
    ) -> BlockLayoutRef {
        let mode = LayoutMode::new(node.clone());

        Rc::new(RefCell::new(Self {
            node,
            parent,
            previous,
            children: Vec::new(),
            font_manager,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            mode,
            cursor_x: 0.0,
            cursor_y: 0.0,
            weight: Weight::NORMAL,
            style: Slant::Upright,
            size: DEFAULT_FONT_SIZE,
            line: Vec::new(),
            display_list: Vec::new(),
        }))
    }

    // return (x, y, width)
    fn calc_pos_and_width(&self) -> (f32, f32, f32) {
        if let Some(parent_weak) = &self.parent
            && let Some(parent_rc) = parent_weak.upgrade()
        {
            let parent = &*parent_rc.borrow();
            let x = parent.x;
            let mut y = parent.y;
            let width = parent.width;

            if let Some(previous_weak) = &self.previous {
                if let Some(previous_rc) = previous_weak.upgrade() {
                    let previous = &*previous_rc.borrow();
                    y = previous.y + previous.height;
                }
            }

            (x, y, width)
        } else {
            // default value
            (DEFAULT_X, DEFAULT_Y, WIDTH - 2.0 * DEFAULT_BROWSER_PADDING)
        }
    }

    fn calc_height(&self) -> f32 {
        match &self.mode {
            LayoutMode::Block => self
                .children
                .iter()
                .map(|child| child.borrow().height)
                .sum(),
            LayoutMode::Inline => self.cursor_y,
        }
    }

    fn layout_block(&mut self, self_rc: BlockLayoutRef) {
        let mut previous_rc: Option<BlockLayoutRef> = None;
        for child in &self.node.borrow().children {
            let next = BlockLayout::new(
                child.clone(),
                Some(Rc::downgrade(&self_rc)),
                match &previous_rc {
                    Some(p_rc) => Some(Rc::downgrade(&p_rc)),
                    None => None,
                },
                self.font_manager.clone(),
            );
            self.children.push(next.clone());
            previous_rc = Some(next);
        }
    }

    fn flush(&mut self) {
        if self.line.is_empty() {
            return;
        }

        let mut max_ascent: f32 = 0.0;
        let mut max_spacing: f32 = 0.0;

        for (_, _, font) in &self.line {
            max_ascent = max_ascent.max(-font.metrics().1.ascent);
            max_spacing = max_spacing.max(font.spacing());
        }

        let baseline = self.y + self.cursor_y + max_ascent;

        for (real_x, word, font) in self.line.drain(..) {
            let x = self.x + real_x;
            let ascent = -font.metrics().1.ascent;
            let y = baseline - ascent;
            self.display_list.push(DisplayItem {
                x,
                y,
                baseline,
                text: word.to_string(),
                font,
            })
        }

        self.cursor_x = 0.0;
        self.cursor_y += max_spacing;
    }

    fn word(&mut self, word: &str) {
        let font = self
            .font_manager
            .borrow_mut()
            .get_font(self.size, self.weight, self.style);

        // Bounding Box
        // let w = font.measure_str(word, None).1.width();
        // let space_w = font.measure_str(" ", None).1.width();

        // Advance Width
        let w = font.measure_str(word, None).0;
        let space_w = font.measure_str(" ", None).0;

        if self.cursor_x + w > self.width {
            self.flush();
        }

        self.line.push((self.cursor_x, word.to_string(), font));

        self.cursor_x += w + space_w;
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

    fn recurse(&mut self, node_rc: HTMLNodeRef) {
        let node = &*node_rc.borrow();
        match &node.data {
            HTMLNodeData::Text(t) => {
                for word in t.text.split_whitespace() {
                    self.word(word);
                }
            }
            HTMLNodeData::Element(e) => {
                let tag = &e.tag;
                self.open_tag(tag);
                for child in &node.children {
                    self.recurse(child.clone());
                }
                self.close_tag(tag);
            }
        }
    }

    fn layout_inline(&mut self) {
        self.cursor_x = 0.0;
        self.cursor_y = 0.0;
        self.weight = Weight::NORMAL;
        self.style = Slant::Upright;
        self.size = DEFAULT_FONT_SIZE;
        self.line.clear();
        self.recurse(self.node.clone());
        self.flush();
    }

    pub fn layout(block_rc: BlockLayoutRef) {
        {
            let block = &mut *block_rc.borrow_mut();
            let (x, y, width) = block.calc_pos_and_width();
            block.x = x;
            block.y = y;
            block.width = width;

            match &block.mode {
                LayoutMode::Block => block.layout_block(block_rc.clone()),
                LayoutMode::Inline => block.layout_inline(),
            }
        }

        {
            for child_rc in &block_rc.borrow().children {
                BlockLayout::layout(child_rc.clone());
            }
        }

        {
            let block = &mut *block_rc.borrow_mut();
            block.height = block.calc_height();
        }
    }

    pub fn paint(&self) -> Vec<DrawCommand> {
        let mut cmds = Vec::new();

        let node = &*self.node.borrow();
        if let HTMLNodeData::Element(e) = &node.data
            && e.tag == "pre"
        {
            let x2 = self.x + self.width;
            let y2 = self.y + self.height;
            cmds.push(DrawCommand::rect(self.x, self.y, x2, y2, Color::GRAY));
        }

        if let LayoutMode::Inline = self.mode {
            for item in &self.display_list {
                cmds.push(DrawCommand::text(
                    item.x,
                    item.y,
                    item.baseline,
                    item.text.to_string(),
                    item.font.clone(),
                ));
            }
        }

        cmds
    }

    pub fn print_tree(&self, depth: usize) {
        let indent = "  ".repeat(depth);

        println!("{}{}", indent, self);

        for child in &self.children {
            child.borrow().print_tree(depth + 1);
        }
    }
}

impl Display for BlockLayout {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "BlockLayout[{}](x={}, y={}, width={}, height={}, node={})",
            self.mode,
            self.x,
            self.y,
            self.width,
            self.height,
            self.node.borrow()
        )
    }
}
