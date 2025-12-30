use crate::dom::attr::Attr;
use crate::dom::html::tree_node::TNodePtr;
use crate::dom::named_node_map::NamedNodeMap;
use crate::dom::node::Node;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// https://dom.spec.whatwg.org/#element
pub(crate) struct Element {
    base: Node,
    tag_name: RefCell<String>,
    attributes: RefCell<NamedNodeMap>,
}

impl Element {
    pub(crate) fn new(parent: Option<TNodePtr>, tag_name: &str, attributes: Vec<Attr>) -> Element {
        Element {
            base: Node::new(parent),
            tag_name: RefCell::new(String::from(tag_name)),
            attributes: RefCell::new(NamedNodeMap::new(attributes)),
        }
    }
}

impl Deref for Element {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Element {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
