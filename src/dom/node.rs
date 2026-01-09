use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};

use crate::dom::{document::Document, element::Element, text::Text};

enum NodeType {
    Document(Document),
    Element(Element),
    Text(Text),
}

struct Node {
    parent: Cell<Option<NonNull<Node>>>,
    prev_sibling: Cell<Option<NonNull<Node>>>,
    next_sibling: Cell<Option<NonNull<Node>>>,
    first_child: Cell<Option<NonNull<Node>>>,
    last_child: Cell<Option<NonNull<Node>>>,
    data: RefCell<NodeType>,
}
