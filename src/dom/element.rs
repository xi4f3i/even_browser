use std::cell::{Cell, Ref, RefCell};

use crate::dom::Attr;

/// https://dom.spec.whatwg.org/#interface-element
pub(crate) struct Element {
    name: RefCell<String>,
    self_closing: Cell<bool>,
    attrs: RefCell<Vec<Attr>>,
}

impl Element {
    pub(crate) fn new(name: String, self_closing: bool, attrs: Vec<Attr>) -> Element {
        Element {
            name: RefCell::new(name),
            self_closing: Cell::new(self_closing),
            attrs: RefCell::new(attrs),
        }
    }

    /// https://dom.spec.whatwg.org/#dom-element-tagname
    pub(crate) fn tag_name(&self) -> Ref<'_, String> {
        self.name.borrow()
    }

    /// https://dom.spec.whatwg.org/#dom-element-attributes
    pub(crate) fn attributes(&self) -> Ref<'_, Vec<Attr>> {
        self.attrs.borrow()
    }
}
