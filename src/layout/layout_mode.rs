use crate::constant::html::BLOCK_ELEMENTS;
use crate::parser::html_node::{HTMLNodeData, HTMLNodeRef};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum LayoutMode {
    Inline,
    Block,
}

impl LayoutMode {
    pub fn new(html_node: HTMLNodeRef) -> Self {
        let html_node = html_node.borrow();
        match &html_node.data {
            HTMLNodeData::Text(_) => LayoutMode::Inline,
            _ => {
                if html_node.children.iter().any(|child_rc| {
                    let child = &*child_rc.borrow();
                    match &child.data {
                        HTMLNodeData::Element(e) => BLOCK_ELEMENTS.contains(&e.tag.as_str()),
                        _ => false,
                    }
                }) {
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

impl Display for LayoutMode {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}",
            match self {
                LayoutMode::Inline => "inline",
                LayoutMode::Block => "block",
            }
        )
    }
}
