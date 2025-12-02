use crate::constant::common::{DOUBLE_QUOTE, EQUALS, EXCLAMATION_MARK, SINGLE_QUOTE, SLASH};
use crate::constant::html::{
    ATTRIBUTE_KEY_HREF, ATTRIBUTE_KEY_REL, ATTRIBUTE_REL_VALUE_STYLESHEET, BODY, HEAD,
    HEAD_ELEMENTS, HTML, LINK, SELF_CLOSING_ELEMENTS, SLASH_HEAD, SLASH_HTML,
};
use crate::parser::html_node::HTMLNodeRef;
use crate::parser::html_node::{HTMLNode, HTMLNodeData};
use std::collections::HashMap;
use std::rc::Rc;
use std::{cell::RefCell, rc::Weak};

#[derive(Debug)]
pub struct HTMLParser {
    body: String,
    unfinished: Vec<Rc<RefCell<HTMLNode>>>,
}

impl HTMLParser {
    pub fn new(body: String) -> Self {
        Self {
            body,
            unfinished: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Option<Rc<RefCell<HTMLNode>>> {
        let mut in_tag = false;
        let mut left: usize = 0;
        let mut right: usize = 0;

        let chars: Vec<char> = std::mem::take(&mut self.body).chars().collect();
        let len = chars.len();

        while right < len {
            match chars[right] {
                '<' => {
                    in_tag = true;

                    let text: String = chars[left..right].iter().collect();
                    if !text.is_empty() {
                        self.add_text(text);
                    }

                    right += 1;
                    left = right;
                }
                '>' => {
                    in_tag = false;
                    let text: String = chars[left..right].iter().collect();
                    self.add_tag(&text);

                    right += 1;
                    left = right;
                }
                _ => {
                    right += 1;
                }
            };
        }

        if !in_tag && left < len {
            let text: String = chars[left..len].iter().collect();
            if !text.is_empty() {
                self.add_text(text);
            }
        }

        self.finish()
    }

    fn finish(&mut self) -> Option<Rc<RefCell<HTMLNode>>> {
        if self.unfinished.is_empty() {
            self.implicit_tags(None);
        }

        while self.unfinished.len() > 1 {
            let node = self
                .unfinished
                .pop()
                .expect("Finish: unfinished pop failed");
            let parent = self
                .unfinished
                .last()
                .expect("Finish: unfinished last failed");
            parent.borrow_mut().children.push(node);
        }

        self.unfinished.pop()
    }

    fn add_text(&mut self, text: String) {
        if text.trim().is_empty() {
            return;
        }

        self.implicit_tags(None);

        let node = HTMLNode::new_text(self.get_parent_weak(), text);

        if let Some(parent_rc) = self.unfinished.last() {
            parent_rc.borrow_mut().children.push(node);
        }
    }

    fn get_parent_weak(&self) -> Option<Weak<RefCell<HTMLNode>>> {
        if let Some(parent_rc) = self.unfinished.last() {
            Some(Rc::downgrade(&parent_rc))
        } else {
            None
        }
    }

    fn add_tag(&mut self, tag_text: &str) {
        let (tag, mut attributes) = self.get_attributes(tag_text);

        if tag.starts_with(EXCLAMATION_MARK) {
            return;
        }

        self.implicit_tags(Some(&tag));

        if tag.starts_with(SLASH) {
            if self.unfinished.len() <= 1 {
                return;
            }

            let node = self
                .unfinished
                .pop()
                .expect("Add tag: unfinished pop failed");
            let parent = self
                .unfinished
                .last()
                .expect("Add tag: unfinished last failed");
            parent.borrow_mut().children.push(node);
        } else if SELF_CLOSING_ELEMENTS.contains(&tag.as_str()) {
            attributes.remove(&SLASH.to_string()); // remove slash attribute of the self-closing tag

            let node = HTMLNode::new_element(self.get_parent_weak(), tag, attributes, true);
            if let Some(parent_rc) = self.unfinished.last() {
                parent_rc.borrow_mut().children.push(node);
            }
        } else {
            self.unfinished.push(HTMLNode::new_element(
                self.get_parent_weak(),
                tag,
                attributes,
                false,
            ));
        }
    }

    fn implicit_tags(&mut self, tag: Option<&str>) {
        loop {
            let tag: &str = tag.unwrap_or("");

            let is_html_root = self
                .unfinished
                .get(0)
                .map_or(false, |n| match &n.borrow().data {
                    HTMLNodeData::Element(e) => e.tag == HTML,
                    _ => false,
                });

            let is_head_second = self
                .unfinished
                .get(1)
                .map_or(false, |n| match &n.borrow().data {
                    HTMLNodeData::Element(e) => e.tag == HEAD,
                    _ => false,
                });

            if self.unfinished.is_empty() && tag != HTML {
                self.add_tag(HTML);
            } else if is_html_root
                && self.unfinished.len() == 1
                && ![HEAD, BODY, SLASH_HTML].contains(&tag)
            {
                if HEAD_ELEMENTS.contains(&tag) {
                    self.add_tag(HEAD);
                } else {
                    self.add_tag(BODY);
                }
            } else if is_html_root
                && is_head_second
                && self.unfinished.len() == 2
                && !HEAD_ELEMENTS.contains(&tag)
                && tag != SLASH_HEAD
            {
                self.add_tag(SLASH_HEAD);
            } else {
                break;
            }
        }
    }

    fn get_attributes(&self, tag_text: &str) -> (String, HashMap<String, String>) {
        let mut parts = tag_text.split_whitespace();

        let tag = match parts.next() {
            Some(t) => t.to_lowercase(),
            None => return (String::new(), HashMap::new()),
        };

        let mut attributes = HashMap::new();

        for attr_pair in parts {
            let mut parts_pair = attr_pair.splitn(2, EQUALS);
            let key = parts_pair
                .next()
                .expect("Get attributes: parts pair next failed")
                .to_lowercase();

            // Simple implementation, to be improved
            if let Some(mut value) = parts_pair.next() {
                if value.len() > 2
                    && (value.starts_with(SINGLE_QUOTE) || value.starts_with(DOUBLE_QUOTE))
                {
                    value = &value[1..value.len() - 1];
                }
                attributes.insert(key, value.to_string());
            } else {
                attributes.insert(key, String::new());
            }
        }

        (tag, attributes)
    }
}

pub fn tree_to_list(tree: HTMLNodeRef, list: &mut Vec<HTMLNodeRef>) {
    list.push(tree.clone());

    for child in tree.borrow().children.iter() {
        tree_to_list(child.clone(), list);
    }
}

pub fn get_links(node: HTMLNodeRef) -> Vec<String> {
    let mut node_list = vec![];
    tree_to_list(node.clone(), &mut node_list);

    node_list
        .iter()
        .filter_map(|node| match &node.borrow().data {
            HTMLNodeData::Element(e) => {
                if e.tag == LINK
                    && e.attributes
                        .get(ATTRIBUTE_KEY_REL)
                        .map_or(false, |v| v == ATTRIBUTE_REL_VALUE_STYLESHEET)
                {
                    e.attributes.get(ATTRIBUTE_KEY_HREF).cloned()
                } else {
                    None
                }
            }
            HTMLNodeData::Text(_) => None,
        })
        .collect()
}
