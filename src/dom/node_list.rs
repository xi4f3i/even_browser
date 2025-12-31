use crate::dom::html::node::TNodePtr;
use std::cell::RefCell;

/// https://dom.spec.whatwg.org/#nodelist
pub(crate) struct NodeList {
    data: RefCell<Vec<TNodePtr>>,
}

impl NodeList {
    pub(crate) fn new() -> NodeList {
        NodeList {
            data: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn append(&self, node: TNodePtr) {
        self.data.borrow_mut().push(node);
    }

    pub(crate) fn last(&self) -> Option<TNodePtr> {
        self.data.borrow().last().copied()
    }

    /// https://dom.spec.whatwg.org/#dom-nodelist-length
    pub(crate) fn length(&self) -> usize {
        self.data.borrow().len()
    }

    /// https://dom.spec.whatwg.org/#dom-nodelist-item
    pub(crate) fn item(&self, idx: usize) -> Option<TNodePtr> {
        self.data.borrow().get(idx).copied()
    }
}
