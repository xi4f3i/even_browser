use crate::dom::html::node::TNodePtr;
use crate::dom::node::Node;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// https://dom.spec.whatwg.org/#interface-characterdata
pub(crate) struct CharacterData {
    base: Node,
    pub(crate) data: RefCell<String>,
}

impl CharacterData {
    pub(crate) fn new(parent: Option<TNodePtr>, data: &str) -> CharacterData {
        CharacterData {
            base: Node::new(parent),
            data: RefCell::new(String::from(data)),
        }
    }

    pub(crate) fn append_data(&self, data: char) {
        self.data.borrow_mut().push(data);
    }
}

impl Deref for CharacterData {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for CharacterData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
