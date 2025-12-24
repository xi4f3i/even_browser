use crate::dom::attribute::Attribute;

/// https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap
#[derive(Debug)]
pub(crate) struct NamedNodeMap {
    data: Vec<Attribute>,
}

impl NamedNodeMap {
    pub(crate) fn new(attributes: Vec<Attribute>) -> NamedNodeMap {
        NamedNodeMap { data: attributes }
    }

    pub(crate) fn set(&mut self, name: &str, value: &str) {
        self.data.push(Attribute {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
}
