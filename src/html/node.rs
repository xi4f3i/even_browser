use std::collections::HashMap;

#[derive(Debug)]
pub struct Element {
    tag: String,
    is_self_closing: bool,
    attributes: HashMap<String, String>,
}

#[derive(Debug)]
pub enum NodeData {
    Comment(String),
    Text(String),
    Element(Element),
}

pub type NodeID = usize;

#[derive(Debug)]
pub struct Node {
    id: NodeID,
    parent: Option<NodeID>,
    pub children: Vec<NodeID>,
    data: NodeData,
}

impl Node {
    pub fn new_comment(id: NodeID, parent: Option<NodeID>, comment: String) -> Self {
        Self {
            id,
            parent,
            children: Vec::new(),
            data: NodeData::Comment(comment),
        }
    }

    pub fn new_element(
        id: NodeID,
        parent: Option<NodeID>,
        tag: String,
        is_self_closing: bool,
        attributes: HashMap<String, String>,
    ) -> Self {
        Self {
            id,
            parent,
            children: Vec::new(),
            data: NodeData::Element(Element {
                tag,
                is_self_closing,
                attributes,
            }),
        }
    }

    pub fn new_text(id: NodeID, parent: Option<NodeID>, text: String) -> Self {
        Self {
            id,
            parent,
            children: Vec::new(),
            data: NodeData::Text(text),
        }
    }

    pub fn get_tag(&self) -> Option<&str> {
        match &self.data {
            NodeData::Element(e) => Some(&e.tag),
            _ => None,
        }
    }

    pub fn get_id(&self) -> NodeID {
        self.id
    }

    pub fn get_parent(&self) -> Option<NodeID> {
        self.parent
    }

    pub fn get_is_self_closing(&self) -> bool {
        match &self.data {
            NodeData::Element(e) => e.is_self_closing,
            _ => false,
        }
    }
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut attrs_vec: Vec<_> = self.attributes.iter().collect();
        attrs_vec.sort_by_key(|(k, _)| *k);

        let mut attr_str = String::new();
        for (k, v) in attrs_vec {
            attr_str.push_str(&format!(" {}=\"{}\"", k, v));
        }

        if self.is_self_closing {
            write!(f, "<{}{}/>", self.tag, attr_str)
        } else {
            write!(f, "<{}{}>", self.tag, attr_str)
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            NodeData::Comment(comment) => write!(f, "<!--{}-->", comment),
            NodeData::Text(text) => write!(f, "{}", text),
            NodeData::Element(element) => write!(f, "{}", element),
        }
    }
}
