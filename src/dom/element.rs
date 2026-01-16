use std::cell::{Ref, RefCell};

use crate::dom::{Atom, Attr};

/// https://dom.spec.whatwg.org/#interface-element
pub(crate) struct Element {
    name: RefCell<Atom>,
    attrs: RefCell<Vec<Attr>>,
}

impl Element {
    pub(crate) fn new(name: Atom, attrs: Vec<Attr>) -> Element {
        Element {
            name: RefCell::new(name),
            attrs: RefCell::new(attrs),
        }
    }

    /// https://dom.spec.whatwg.org/#dom-element-tagname
    pub(crate) fn tag_name(&self) -> Ref<'_, Atom> {
        self.name.borrow()
    }

    /// https://dom.spec.whatwg.org/#dom-element-attributes
    pub(crate) fn attributes(&self) -> Ref<'_, Vec<Attr>> {
        self.attrs.borrow()
    }
}
