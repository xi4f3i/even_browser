use crate::dom::named_node_map::NamedNodeMap;

/// https://developer.mozilla.org/en-US/docs/Web/API/Element
#[derive(Debug)]
pub(crate) struct Element {
    tag_name: String,
    /// https://developer.mozilla.org/en-US/docs/Web/API/Element/attributes
    attributes: NamedNodeMap,
}

impl Element {
    pub(crate) fn new(tag_name: &str, attributes: NamedNodeMap) -> Element {
        Element {
            tag_name: tag_name.to_string(),
            attributes,
        }
    }

    pub(crate) fn tag_name(&self) -> &str {
        &self.tag_name
    }
}
