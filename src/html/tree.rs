use crate::html::node::{Node, NodeID};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;

#[derive(Debug, Default)]
pub struct DOMTree {
    nodes: Vec<Node>,
    pub root: Option<NodeID>,
    doc_type: Option<NodeID>,
}

impl DOMTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            doc_type: None,
        }
    }

    pub fn add_doc_type(&mut self, parent_id: Option<NodeID>, doc_type: String) -> NodeID {
        let id = self.nodes.len();
        let comment = Node::new_doc_type(id, parent_id, doc_type);

        self.append_node(comment);

        self.doc_type = Some(id);

        id
    }

    pub fn add_comment(&mut self, parent_id: Option<NodeID>, comment: String) -> NodeID {
        let id = self.nodes.len();
        let comment = Node::new_comment(id, parent_id, comment);

        self.append_node(comment);

        id
    }

    pub fn add_element(
        &mut self,
        parent_id: Option<NodeID>,
        tag: String,
        is_self_closing: bool,
        attributes: HashMap<String, String>,
    ) -> NodeID {
        let id = self.nodes.len();
        let element = Node::new_element(id, parent_id, tag, is_self_closing, attributes);

        self.append_node(element);

        id
    }

    pub fn add_text(&mut self, parent_id: Option<NodeID>, text: String) -> NodeID {
        let id = self.nodes.len();
        let text = Node::new_text(id, parent_id, text);

        self.append_node(text);
        id
    }

    fn append_node(&mut self, node: Node) {
        if let Some(parent_id) = node.get_parent() {
            self.nodes[parent_id].children.push(node.get_id());
        }

        self.nodes.push(node);
    }

    pub fn get_node(&self, id: NodeID) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn print(&self) {
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("log/dom_tree.html")
            && let Some(id) = self.root
        {
            if let Some(doc_type_id) = self.doc_type
                && let Some(doc_type) = self.get_node(doc_type_id)
            {
                let _ = writeln!(&file, "{}", doc_type);
            }
            self.print_node(id, 0, &file);
        }
    }

    fn print_node(&self, id: usize, depth: usize, mut file: &File) {
        let indent = "  ".repeat(depth);

        if let Some(node) = self.get_node(id) {
            let _ = writeln!(file, "{}{}", indent, node);

            for child in &node.children {
                self.print_node(*child, depth + 1, file);
            }

            if !node.get_is_self_closing()
                && let Some(tag) = node.get_tag()
            {
                let _ = writeln!(file, "{}</{}>", indent, tag);
            }
        }
    }
}
