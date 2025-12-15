use std::ptr::NonNull;

use crate::dom::{
    named_node_map::NamedNodeMap,
    node::{Node, NodeBox, NodePtr},
};

const SELF_CLOSING_TAGS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const HEAD_TAGS: [&str; 9] = [
    "base", "basefont", "bgsound", "noscript", "link", "meta", "title", "style", "script",
];

const HEAD_BODY_SLASH_HTML: [&str; 3] = ["head", "body", "/html"];

struct HTMLParser {
    input: Vec<char>,
    idx: usize,
    unfinished: Vec<NodeBox>,
}

impl HTMLParser {
    fn new(input: &str) -> HTMLParser {
        HTMLParser {
            input: input.chars().collect(),
            idx: 0,
            unfinished: vec![Node::new_document()],
        }
    }

    fn parse(&mut self) -> NodeBox {
        while self.idx < self.input.len() {
            let c = self.input[self.idx];
            match c {
                '<' => {
                    self.idx += 1;
                    self.parse_tag();
                }
                '>' => {
                    self.idx += 1;
                    self.parse_text();
                }
                _ => {
                    self.idx += 1;
                }
            }
        }

        self.finish()
    }

    fn text(&mut self) -> Result<String, ()> {
        let start_idx = self.idx;

        while self.idx < self.input.len() {
            let c = self.input[self.idx];
            if c == '<' {
                break;
            }
            self.idx += 1;
        }

        if start_idx >= self.idx {
            return Err(());
        }

        let text: String = self.input[start_idx..self.idx].iter().collect();

        if text.trim().is_empty() {
            return Err(());
        }

        Ok(text)
    }

    fn parse_text(&mut self) {
        if let Ok(text) = self.text() {
            self.implicit_tags(None);

            let node = Node::new_text(self.last_unfinished_ptr(), &text);

            if let Some(parent) = self.unfinished.last_mut() {
                parent.append_child(node);
            }
        }
    }

    fn implicit_tags(&mut self, tag_name: Option<&str>) {
        let tag_name = tag_name.unwrap_or("");

        loop {
            let open_tag_names: Vec<&str> = self
                .unfinished
                .iter()
                .map(|n| n.get_element())
                .flatten()
                .map(|e| e.tag_name())
                .collect();

            if open_tag_names.is_empty() && tag_name != "html" {
                self.add_tag("html", NamedNodeMap::new());

                continue;
            }

            let is_html_root = open_tag_names.first().map_or(false, |n| *n == "html");

            if is_html_root
                && open_tag_names.len() == 1
                && !HEAD_BODY_SLASH_HTML.contains(&tag_name)
            {
                if HEAD_TAGS.contains(&tag_name) {
                    self.add_tag("head", NamedNodeMap::new());
                } else {
                    self.add_tag("body", NamedNodeMap::new());
                }

                continue;
            }

            let is_head_second = open_tag_names.get(1).map_or(false, |n| *n == "head");

            if is_html_root
                && is_head_second
                && open_tag_names.len() == 2
                && !HEAD_TAGS.contains(&tag_name)
                && tag_name != "/head"
            {
                self.add_tag("/head", NamedNodeMap::new());

                continue;
            }

            break;
        }
    }

    fn tag_name(&mut self) -> String {
        let start_idx = self.idx;

        while self.idx < self.input.len() {
            let c = self.input[self.idx];

            if c.is_whitespace() || c == '>' || c == '/' {
                break;
            }

            self.idx += 1;
        }

        self.input[start_idx..self.idx]
            .iter()
            .collect::<String>()
            .to_lowercase()
    }

    fn whitespace(&mut self) {
        while self.idx < self.input.len() && self.input[self.idx].is_whitespace() {
            self.idx += 1;
        }
    }

    fn attribute_name(&mut self) -> String {
        let start_idx = self.idx;

        while self.idx < self.input.len() {
            let c = self.input[self.idx];

            if c == '/' || c == '>' || c == '=' || c.is_whitespace() {
                break;
            }

            self.idx += 1;
        }

        self.input[start_idx..self.idx]
            .iter()
            .collect::<String>()
            .to_lowercase()
    }

    fn attribute_value(&mut self) -> String {
        if self.idx >= self.input.len() || self.input[self.idx] != '=' {
            return String::new();
        }

        self.idx += 1;

        if self.idx >= self.input.len() {
            return String::new();
        }

        let (start_idx, end_char) = match self.input[self.idx] {
            '"' => {
                self.idx += 1;
                (self.idx, '"')
            }
            '\'' => {
                self.idx += 1;
                (self.idx, '\'')
            }
            _ => (self.idx, ' '),
        };

        while self.idx < self.input.len() {
            let c = self.input[self.idx];

            if end_char == ' ' {
                if c.is_whitespace() {
                    break;
                }
            } else if c == end_char {
                break;
            }

            self.idx += 1;
        }

        let end_idx = if end_char == ' ' {
            self.idx
        } else {
            self.idx += 1;
            self.idx - 1
        };

        self.input[start_idx..end_idx].iter().collect::<String>()
    }

    fn attribute(&mut self, attributes: &mut NamedNodeMap) {
        self.whitespace();
        let name = self.attribute_name();
        let value = self.attribute_value();
        self.whitespace();

        if !name.is_empty() {
            attributes.set(&name, &value);
        }
    }

    fn attributes(&mut self) -> NamedNodeMap {
        let mut attributes = NamedNodeMap::new();

        while self.idx < self.input.len()
            && self.input[self.idx] != '>'
            && self.input[self.idx] != '/'
        {
            self.attribute(&mut attributes);
        }

        attributes
    }

    fn parse_tag(&mut self) {
        if self.idx >= self.input.len() {
            return;
        }

        if self.input[self.idx] == '!' {
            // Handle comments or DOCTYPE
            return;
        }

        if self.input[self.idx] == '/' {
            // Closing tag
            self.idx += 1;

            if let Some(last) = self.unfinished.pop() {
                if let Some(parent) = self.unfinished.last_mut() {
                    parent.append_child(last);
                }
            }

            return;
        }

        if !self.input[self.idx].is_ascii_alphabetic() {
            // Invalid tag name start
            return;
        }

        let tag_name = self.tag_name();
        let attributes = self.attributes();

        self.implicit_tags(Some(&tag_name));

        self.add_tag(&tag_name, attributes);
    }

    fn add_tag(&mut self, tag_name: &str, attributes: NamedNodeMap) {
        println!("add_tag tag_name={}", tag_name);

        let node = Node::new_element(self.last_unfinished_ptr(), tag_name, attributes);

        if SELF_CLOSING_TAGS.contains(&tag_name) {
            if let Some(parent) = self.unfinished.last_mut() {
                parent.append_child(node);
            }
        } else {
            self.unfinished.push(node);
        }
    }

    fn last_unfinished_ptr(&self) -> Option<NodePtr> {
        self.unfinished.last().map(|n| NonNull::from(n.as_ref()))
    }

    fn finish(&mut self) -> NodeBox {
        if self.unfinished.is_empty() {
            self.implicit_tags(None);
        }

        while self.unfinished.len() > 1 {
            let node = self.unfinished.pop();
            let parent = self.unfinished.last_mut();

            if let Some(node) = node
                && let Some(parent) = parent
            {
                parent.append_child(node);
            }
        }

        self.unfinished.pop().unwrap_or(Node::new_document())
    }
}

#[cfg(test)]
mod tests {
    use crate::dom::node::print_node_tree;

    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let html =
            "<html><head><title>Test</title></head><body><h1>Hello, World!</h1></body></html>";
        let mut parser = HTMLParser::new(html);
        let document = parser.parse();
        print_node_tree(&document, 0);
        assert_eq!(document.child_nodes().len(), 1);
        let html_node = &document.child_nodes()[0];
        assert_eq!(html_node.get_element().unwrap().tag_name(), "html");
        let head_node = &html_node.child_nodes()[0];
        assert_eq!(head_node.get_element().unwrap().tag_name(), "head");
        let body_node = &html_node.child_nodes()[1];
        assert_eq!(body_node.get_element().unwrap().tag_name(), "body");
    }
}
