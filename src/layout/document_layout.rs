use crate::constant::layout::{DEFAULT_WIDTH, DEFAULT_X, DEFAULT_Y};
use crate::layout::block_layout::{BlockLayout, BlockLayoutRef};
use crate::layout::font_manager::{FontManager, FontManagerRef};
use crate::parser::html_node::HTMLNodeRef;
use std::cell::RefCell;
use std::fmt::{Display, Formatter, Result};
use std::rc::Rc;

pub type DocumentLayoutRef = Rc<RefCell<DocumentLayout>>;

#[derive(Debug)]
pub struct DocumentLayout {
    node: HTMLNodeRef,
    pub child: Option<BlockLayoutRef>,
    font_manager: FontManagerRef,
    x: f32,
    y: f32,
    pub height: f32,
    width: f32,
}

impl DocumentLayout {
    pub fn new(node: HTMLNodeRef) -> DocumentLayoutRef {
        Rc::new(RefCell::new(Self {
            node,
            child: None,
            font_manager: FontManager::new(),
            x: DEFAULT_X,
            y: DEFAULT_Y,
            height: 0.0,
            width: DEFAULT_WIDTH,
        }))
    }

    pub fn layout(&mut self) {
        let child_rc = BlockLayout::new(self.node.clone(), None, None, self.font_manager.clone());
        self.child = Some(child_rc.clone());
        BlockLayout::layout(child_rc.clone());
        self.height = child_rc.borrow().height;
    }

    pub fn print_tree(&self, depth: usize) {
        let indent = "  ".repeat(depth);

        println!("{}{}", indent, self);

        if let Some(child_rc) = &self.child {
            child_rc.borrow().print_tree(depth + 1);
        }
    }
}

impl Display for DocumentLayout {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "DocumentLayout()")
    }
}
