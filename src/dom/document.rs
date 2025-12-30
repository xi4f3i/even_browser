use crate::dom::node::Node;
use std::ops::{Deref, DerefMut};

/// https://dom.spec.whatwg.org/#document
pub(crate) struct Document {
    base: Node,
}

impl Document {
    pub(crate) fn new() -> Document {
        Document {
            base: Node::new(None),
        }
    }
}

impl Deref for Document {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Document {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
