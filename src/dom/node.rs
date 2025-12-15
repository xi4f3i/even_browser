use std::ptr;

use crate::dom::{document::Document, element::Element, named_node_map::NamedNodeMap, text::Text};

pub(crate) type NodePtr = ptr::NonNull<Node>;
pub(crate) type NodeBox = Box<Node>;

#[derive(Debug)]
enum NodeSubtype {
    Document(Document),
    Element(Element),
    Text(Text),
}

/// https://developer.mozilla.org/en-US/docs/Web/API/Node
#[derive(Debug)]
pub(crate) struct Node {
    parent: Option<NodePtr>,
    children: Vec<NodeBox>,
    subtype: NodeSubtype,
}

impl Node {
    pub(crate) fn new_document() -> NodeBox {
        Box::new(Node {
            parent: None,
            children: Vec::new(),
            subtype: NodeSubtype::Document(Document::new()),
        })
    }

    pub(crate) fn new_element(
        parent: Option<NodePtr>,
        tag_name: &str,
        attributes: NamedNodeMap,
    ) -> NodeBox {
        Box::new(Node {
            parent,
            children: Vec::new(),
            subtype: NodeSubtype::Element(Element::new(tag_name, attributes)),
        })
    }

    pub(crate) fn new_text(parent: Option<NodePtr>, data: &str) -> NodeBox {
        Box::new(Node {
            parent,
            children: Vec::new(),
            subtype: NodeSubtype::Text(Text::new(data)),
        })
    }

    pub(crate) fn get_document(&self) -> Option<&Document> {
        match &self.subtype {
            NodeSubtype::Document(d) => Some(d),
            _ => None,
        }
    }

    pub(crate) fn get_element(&self) -> Option<&Element> {
        match &self.subtype {
            NodeSubtype::Element(e) => Some(e),
            _ => None,
        }
    }

    pub(crate) fn get_text(&self) -> Option<&Text> {
        match &self.subtype {
            NodeSubtype::Text(t) => Some(t),
            _ => None,
        }
    }

    pub(crate) fn get_parent(&self) -> Option<&Node> {
        self.parent.map(|node_ptr| unsafe { node_ptr.as_ref() })
    }

    /// TODO: https://developer.mozilla.org/en-US/docs/Web/API/Node/appendChild
    pub(crate) fn append_child(&mut self, child: NodeBox) {
        self.children.push(child);
    }

    pub(crate) fn child_nodes(&self) -> &Vec<NodeBox> {
        &self.children
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.subtype {
            NodeSubtype::Document(_) => write!(f, "#document"),
            NodeSubtype::Element(e) => write!(f, "#{}", e.tag_name()),
            NodeSubtype::Text(t) => write!(f, "#text: {}", t.data()),
        }
    }
}

pub(crate) fn print_node_tree(node: &Node, depth: usize) {
    for _ in 0..depth {
        print!("  ");
    }
    println!("{}", node);

    for child in &node.children {
        print_node_tree(child, depth + 1);
    }
}
