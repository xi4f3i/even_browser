use std::cell::RefCell;

/// https://dom.spec.whatwg.org/#attr
pub(crate) struct Attr {
    name: RefCell<String>,
    value: RefCell<String>,
}

impl Attr {
    pub(crate) fn new(name: &str, value: &str) -> Attr {
        Attr {
            name: RefCell::new(String::from(name)),
            value: RefCell::new(String::from(value)),
        }
    }
}
