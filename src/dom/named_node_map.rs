use crate::dom::attr::Attr;
use std::cell::RefCell;

/// https://dom.spec.whatwg.org/#namednodemap
pub(crate) struct NamedNodeMap {
    data: RefCell<Vec<Attr>>,
}

impl NamedNodeMap {
    pub(crate) fn new(data: Vec<Attr>) -> NamedNodeMap {
        NamedNodeMap {
            data: RefCell::new(data),
        }
    }
}
