use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use std::rc::{Rc, Weak};

use crate::constant::SELF_CLOSING_TAGS;

#[derive(Debug)]
pub struct HTMLTextData {
    pub text: String,
}

#[derive(Debug)]
pub struct HTMLElementData {
    pub tag: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug)]
pub enum HTMLNodeData {
    Text(HTMLTextData),
    Element(HTMLElementData),
}

#[derive(Debug)]
pub struct HTMLNode {
    pub data: HTMLNodeData,
    pub parent: Option<Weak<RefCell<HTMLNode>>>,
    pub children: Vec<Rc<RefCell<HTMLNode>>>,
}

impl HTMLNode {
    pub fn new_text(parent: Option<Weak<RefCell<HTMLNode>>>, text: String) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            data: HTMLNodeData::Text(HTMLTextData { text }),
            parent,
            children: Vec::new(),
        }))
    }

    pub fn new_element(
        parent: Option<Weak<RefCell<HTMLNode>>>,
        tag: String,
        attributes: HashMap<String, String>,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            data: HTMLNodeData::Element(HTMLElementData { tag, attributes }),
            parent,
            children: Vec::new(),
        }))
    }

    pub fn print_tree(&self, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}{}", indent, self);
        for child in &self.children {
            child.borrow().print_tree(depth + 1);
        }
    }
}

impl Display for HTMLNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.data {
            HTMLNodeData::Text(t) => write!(f, "{:?}", t.text),
            HTMLNodeData::Element(e) => {
                let mut attrs_vec: Vec<_> = e.attributes.iter().collect();
                attrs_vec.sort_by_key(|(k, _)| *k);

                let mut attr_str = String::new();
                for (k, v) in attrs_vec {
                    attr_str.push_str(&format!(" {}=\"{}\"", k, v));
                }

                if SELF_CLOSING_TAGS.contains(&e.tag.as_str()) {
                    write!(f, "<{}{}/>", e.tag, attr_str)
                } else {
                    write!(f, "<{}{}>", e.tag, attr_str)
                }
            }
        }
    }
}
