use crate::dom::attr::Attr;
use crate::dom::html::node::{TNode, TNodePtr};
use crate::dom::named_node_map::NamedNodeMap;
use crate::dom::node::Node;
use crate::dom::text::Text;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

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

    pub(crate) fn insert_character(&self, ch: char, self_ptr: TNodePtr) {
        if let Some(child) = self.last_child()
            && let TNode::Text(text) = unsafe { child.as_ref() }
        {
            text.append_data(ch);
            return;
        }

        let node = unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(TNode::Text(Text::new(
                Some(self_ptr),
                &ch.to_string(),
            )))))
        };

        self.append_child(node);
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
