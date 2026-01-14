use std::{
    cell::{Cell, Ref, RefCell},
    ptr::NonNull,
};

use crate::dom::{Atom, Attr, Document, Element, Text};

pub(crate) type NodePtr = NonNull<Node>;

pub(crate) enum NodeType {
    Document(Document),
    Element(Element),
    Text(Text),
}

/// https://dom.spec.whatwg.org/#interface-node
pub(crate) struct Node {
    parent: Cell<Option<NodePtr>>,
    prev_sibling: Cell<Option<NodePtr>>,
    next_sibling: Cell<Option<NodePtr>>,
    pub(crate) first_child: Cell<Option<NodePtr>>,
    last_child: Cell<Option<NodePtr>>,
    data: RefCell<NodeType>,
}

impl Node {
    pub(crate) fn new_doc() -> NodePtr {
        unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                parent: Cell::new(None),
                prev_sibling: Cell::new(None),
                next_sibling: Cell::new(None),
                first_child: Cell::new(None),
                last_child: Cell::new(None),
                data: RefCell::new(NodeType::Document(Document::new())),
            })))
        }
    }

    pub(crate) fn new_elem(
        parent: Option<NodePtr>,
        prev_sibling: Option<NodePtr>,
        name: Atom,
        self_closing: bool,
        attrs: Vec<Attr>,
    ) -> NodePtr {
        unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                parent: Cell::new(parent),
                prev_sibling: Cell::new(prev_sibling),
                next_sibling: Cell::new(None),
                first_child: Cell::new(None),
                last_child: Cell::new(None),
                data: RefCell::new(NodeType::Element(Element::new(name, self_closing, attrs))),
            })))
        }
    }

    pub(crate) fn new_text(
        parent: Option<NodePtr>,
        prev_sibling: Option<NodePtr>,
        ch: char,
    ) -> NodePtr {
        unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                parent: Cell::new(parent),
                prev_sibling: Cell::new(prev_sibling),
                next_sibling: Cell::new(None),
                first_child: Cell::new(None),
                last_child: Cell::new(None),
                data: RefCell::new(NodeType::Text(Text::new(ch))),
            })))
        }
    }

    pub(crate) fn as_ptr(&self) -> NodePtr {
        NonNull::from(self)
    }

    /// https://dom.spec.whatwg.org/#dom-node-nodetype
    pub(crate) fn node_type(&self) -> Ref<'_, NodeType> {
        self.data.borrow()
    }

    /// https://dom.spec.whatwg.org/#dom-node-appendchild
    pub(crate) fn append_child(&self, node: NodePtr) {
        match self.last_child.get() {
            Some(p) => unsafe {
                p.as_ref().set_next_sibling(node);
            },
            None => {
                self.first_child.set(Some(node));
            }
        };

        self.last_child.set(Some(node));
    }

    /// https://dom.spec.whatwg.org/#dom-node-haschildnodes
    pub(crate) fn has_child_nodes(&self) -> bool {
        match self.last_child.get() {
            Some(_) => true,
            None => false,
        }
    }

    /// https://dom.spec.whatwg.org/#dom-node-nextsibling
    pub(crate) fn next_sibling(&self) -> Option<NodePtr> {
        self.next_sibling.get()
    }

    pub(crate) fn set_next_sibling(&self, node: NodePtr) {
        self.next_sibling.set(Some(node));
    }

    /// https://dom.spec.whatwg.org/#dom-node-firstchild
    pub(crate) fn first_child(&self) -> Option<NodePtr> {
        self.first_child.get()
    }

    pub(crate) fn set_first_child(&self, node: NodePtr) {
        self.first_child.set(Some(node));
    }

    /// https://dom.spec.whatwg.org/#dom-node-lastchild
    pub(crate) fn last_child(&self) -> Option<NodePtr> {
        self.last_child.get()
    }

    pub(crate) fn set_last_child(&self, node: NodePtr) {
        self.last_child.set(Some(node));
    }
}
