use std::cell::{Ref, RefCell};

/// https://dom.spec.whatwg.org/#interface-text
/// https://dom.spec.whatwg.org/#characterdata
pub(crate) struct Text {
    data: RefCell<String>,
}

impl Text {
    pub(crate) fn new(ch: char) -> Text {
        Text {
            data: RefCell::new(String::from(ch)),
        }
    }

    /// https://dom.spec.whatwg.org/#dom-characterdata-appenddata
    pub(crate) fn append_data(&self, data: char) {
        self.data.borrow_mut().push(data);
    }

    pub(crate) fn data(&self) -> Ref<'_, String> {
        self.data.borrow()
    }
}
