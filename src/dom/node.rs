use crate::dom::html::node::TNodePtr;
use crate::dom::node_list::NodeList;
use std::cell::{Cell, RefCell};

/// https://dom.spec.whatwg.org/#node
pub(crate) struct Node {
    parent_node: Cell<Option<TNodePtr>>,
    child_nodes: RefCell<NodeList>,
}

impl Node {
    pub(crate) fn new(parent: Option<TNodePtr>) -> Node {
        Node {
            parent_node: Cell::new(parent),
            child_nodes: RefCell::new(NodeList::new()),
        }
    }

    pub(crate) fn set_parent(&self, parent: Option<TNodePtr>) {
        self.parent_node.set(parent);
    }

    /// https://dom.spec.whatwg.org/#dom-node-appendchild
    pub(crate) fn append_child(&self, node: TNodePtr) {
        self.child_nodes.borrow().append(node);
    }

    /// https://dom.spec.whatwg.org/#dom-node-lastchild
    pub(crate) fn last_child(&self) -> Option<TNodePtr> {
        self.child_nodes.borrow().last()
    }
}
