use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use std::rc::{Rc, Weak};

pub type HTMLNodeRef = Rc<RefCell<HTMLNode>>;
pub type HTMLNodeWeakRef = Weak<RefCell<HTMLNode>>;

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

pub type HTMLNodeStyle = HashMap<String, String>;

#[derive(Debug)]
pub struct HTMLNode {
    pub data: HTMLNodeData,
    pub parent: Option<HTMLNodeWeakRef>,
    pub children: Vec<HTMLNodeRef>,
    pub is_self_closing_tag: bool,
    pub style: HTMLNodeStyle,
}

impl HTMLNode {
    pub fn new_text(parent: Option<HTMLNodeWeakRef>, text: String) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            data: HTMLNodeData::Text(HTMLTextData { text }),
            parent,
            children: Vec::new(),
            is_self_closing_tag: false,
            style: HashMap::new(),
        }))
    }

    pub fn new_element(
        parent: Option<HTMLNodeWeakRef>,
        tag: String,
        attributes: HashMap<String, String>,
        is_self_closing_tag: bool,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            data: HTMLNodeData::Element(HTMLElementData { tag, attributes }),
            parent,
            children: Vec::new(),
            is_self_closing_tag,
            style: HashMap::new(),
        }))
    }

    pub fn print_tree(&self, depth: usize) {
        let indent = "  ".repeat(depth);

        println!("{}{}", indent, self);

        for child in &self.children {
            child.borrow().print_tree(depth + 1);
        }

        if !self.is_self_closing_tag {
            match &self.data {
                HTMLNodeData::Element(e) => {
                    println!("{}</{}>", indent, e.tag);
                }
                _ => {}
            }
        }
    }
}

impl Display for HTMLNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.data {
            HTMLNodeData::Text(t) => write!(f, "{}", t.text),
            HTMLNodeData::Element(e) => {
                let mut attrs_vec: Vec<_> = e.attributes.iter().collect();
                attrs_vec.sort_by_key(|(k, _)| *k);

                let mut attr_str = String::new();
                for (k, v) in attrs_vec {
                    attr_str.push_str(&format!(" {}=\"{}\"", k, v));
                }

                if self.is_self_closing_tag {
                    write!(f, "<{}{}/>", e.tag, attr_str)
                } else {
                    write!(f, "<{}{}>", e.tag, attr_str)
                }
            }
        }
    }
}
