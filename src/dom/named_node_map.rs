use std::collections::HashMap;

/// https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap
#[derive(Debug)]
pub(crate) struct NamedNodeMap {
    data: HashMap<String, String>,
}

impl NamedNodeMap {
    pub(crate) fn new() -> NamedNodeMap {
        NamedNodeMap {
            data: HashMap::new(),
        }
    }

    pub(crate) fn set(&mut self, name: &str, value: &str) {
        self.data.insert(name.to_string(), value.to_string());
    }
}
