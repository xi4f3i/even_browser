use crate::constant::{HEAD_TAGS, SELF_CLOSING_TAGS};
use std::collections::HashMap;

pub type NodePtr = Box<Node>;

#[derive(Debug)]
pub enum Node {
    Element(Element),
    Text(Text),
}

pub trait NodeTrait {
    fn children(&self) -> &Vec<NodePtr>;
    fn children_mut(&mut self) -> &mut Vec<NodePtr>;
    fn parent(&self) -> Option<&NodePtr>;
    fn parent_mut(&mut self) -> Option<&mut NodePtr>;
}

#[derive(Default, Debug)]
pub struct Text {
    pub text: String,
    // Note: Parent is omitted or handled via Tree structure to simplify ownership
    // For simplicity in this direct port, we focus on the tree structure (children)
    // and rely on the parser's logic (unfinished stack) for building.
}

#[derive(Default, Debug)]
pub struct Element {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<NodePtr>,
    // Note: Parent is omitted for the same reason as Text
}

impl Element {
    pub fn new(tag: String, attributes: HashMap<String, String>) -> Self {
        Element {
            tag,
            attributes,
            children: Vec::new(),
        }
    }
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let attr_str: String = self
            .attributes
            .iter()
            .map(|(k, v)| format!(" {}=\"{}\"", k, v))
            .collect();
        write!(f, "<{}{}>", self.tag, attr_str)
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.text)
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Element(e) => e.fmt(f),
            Node::Text(t) => t.fmt(f),
        }
    }
}

pub fn print_tree(node: &Node, indent: usize) {
    println!("{:width$}{}", "", node, width = indent);

    let children = match node {
        Node::Element(e) => &e.children,
        Node::Text(_) => return,
    };

    for child in children {
        print_tree(child.as_ref(), indent + 2);
    }
}

#[derive(Debug)]
pub struct HTMLParser {
    body: String,
    unfinished: Vec<NodePtr>,
}

impl HTMLParser {
    pub fn new(body: String) -> Self {
        Self {
            body,
            unfinished: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> NodePtr {
        let mut text = String::new();
        let mut in_tag = false;

        for c in self.body.chars().collect::<Vec<char>>() {
            match c {
                '<' => {
                    in_tag = true;
                    if !text.is_empty() {
                        self.add_text(text.as_str());
                    }
                    text.clear();
                }
                '>' => {
                    in_tag = false;
                    self.add_tag(text.as_str());
                    text.clear();
                }
                _ => text.push(c),
            }
        }

        if !in_tag && !text.is_empty() {
            self.add_text(text.as_str());
        }

        self.finish()
    }

    fn add_text(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }

        self.implicit_tags(None);

        if let Some(Node::Element(parent_elem)) = self.unfinished.last_mut().map(|n| n.as_mut()) {
            let node = Node::Text(Text {
                text: text.trim().to_string(),
            });
            parent_elem.children.push(Box::new(node));
        }
    }

    fn get_attributes(&self, text: &str) -> (String, HashMap<String, String>) {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.is_empty() {
            return (String::new(), HashMap::new());
        }

        let tag = parts[0].to_lowercase();
        let mut attributes = HashMap::new();

        for attr_pair in parts.iter().skip(1) {
            let mut parts_pair = attr_pair.splitn(2, '=');
            let key = parts_pair.next().unwrap().to_lowercase();

            if let Some(mut value) = parts_pair.next() {
                if value.len() > 2 && (value.starts_with('\'') || value.starts_with('"')) {
                    value = &value[1..value.len() - 1];
                }
                attributes.insert(key, value.to_owned());
            } else {
                attributes.insert(key, String::new());
            }
        }

        (tag, attributes)
    }

    fn add_tag(&mut self, tag: &str) {
        let (tag, attributes) = self.get_attributes(tag);

        if tag.starts_with("!") {
            return;
        }

        self.implicit_tags(Some(&tag));

        if tag.starts_with("/") {
            if self.unfinished.len() == 1 {
                return;
            }

            if let Some(node) = self.unfinished.pop() {
                if let Some(Node::Element(parent_elem)) =
                    self.unfinished.last_mut().map(|n| n.as_mut())
                {
                    parent_elem.children.push(node);
                }
            }
        } else if SELF_CLOSING_TAGS.contains(&tag.as_str()) {
            if let Some(Node::Element(parent_elem)) = self.unfinished.last_mut().map(|n| n.as_mut())
            {
                let node = Node::Element(Element::new(tag, attributes));
                parent_elem.children.push(Box::new(node));
            }
        } else {
            let node = Node::Element(Element::new(tag, attributes));
            self.unfinished.push(Box::new(node));
        }
    }

    fn implicit_tags(&mut self, tag: Option<&str>) {
        loop {
            let open_tags: Vec<&str> = self
                .unfinished
                .iter()
                .filter_map(|node_ptr| {
                    if let Node::Element(elem) = node_ptr.as_ref() {
                        Some(elem.tag.as_str())
                    } else {
                        None
                    }
                })
                .collect();

            let current_tag = tag.unwrap_or("");

            if open_tags.is_empty() && current_tag != "html" {
                self.add_tag("html");
            } else if open_tags == vec!["html"]
                && current_tag != "head"
                && current_tag != "body"
                && current_tag != "/html"
            {
                if HEAD_TAGS.contains(&current_tag) {
                    self.add_tag("head");
                } else {
                    self.add_tag("body");
                }
            } else if open_tags == vec!["html", "head"]
                && current_tag != "/head"
                && !HEAD_TAGS.contains(&current_tag)
            {
                self.add_tag("/head");
            } else {
                break;
            }
        }
    }

    pub fn finish(&mut self) -> NodePtr {
        if self.unfinished.is_empty() {
            self.implicit_tags(None);
        }

        while self.unfinished.len() > 1 {
            let node = self.unfinished.pop().unwrap();
            if let Some(Node::Element(parent_elem)) = self.unfinished.last_mut().map(|n| n.as_mut())
            {
                parent_elem.children.push(node);
            }
        }

        self.unfinished.pop().unwrap()
    }
}
