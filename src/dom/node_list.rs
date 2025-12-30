use crate::dom::html::tree_node::{TNodeBox, TNodePtr};
use std::cell::RefCell;
use std::ptr;

/// https://dom.spec.whatwg.org/#nodelist
pub(crate) struct NodeList {
    data: RefCell<Vec<TNodeBox>>,
}

impl NodeList {
    pub(crate) fn new() -> NodeList {
        NodeList {
            data: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn append(&self, node: TNodeBox) {
        self.data.borrow_mut().push(node);
    }

    pub(crate) fn last(&self) -> Option<TNodePtr> {
        self.data
            .borrow_mut()
            .last_mut()
            .map(|node| ptr::NonNull::from(&mut **node))
    }

    /// https://dom.spec.whatwg.org/#dom-nodelist-length
    pub(crate) fn length(&self) -> usize {
        self.data.borrow().len()
    }

    /// https://dom.spec.whatwg.org/#dom-nodelist-item
    pub(crate) fn item(&self, idx: usize) -> Option<TNodePtr> {
        self.data
            .borrow_mut()
            .get_mut(idx)
            .map(|node| ptr::NonNull::from(&mut **node))
    }
}
