use std::collections::HashMap;
use std::rc::Rc;
use std::{cell::RefCell, rc::Weak};

use crate::{
    constant::{HEAD_TAGS, SELF_CLOSING_TAGS},
    parser::html_node::{HTMLNode, HTMLNodeData},
};

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

    pub fn parse(&mut self) -> Rc<RefCell<HTMLNode>> {
        let mut in_tag = false;
        let mut left: usize = 0;
        let mut right: usize = 0;

        let chars: Vec<char> = std::mem::take(&mut self.body).chars().collect();
        let len = chars.len();

        while right < len {
            let c = chars[right];

            match c {
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

    fn finish(&mut self) -> Rc<RefCell<HTMLNode>> {
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

        self.unfinished
            .pop()
            .expect("Finish: unfinished pop root node failed")
    }

    fn add_text(&mut self, text: String) {
        if text.trim().is_empty() {
            return;
        }

        self.implicit_tags(None);

        let parent_rc = self.unfinished.last().expect("Should have parent").clone();

        let node = HTMLNode::new_text(Some(Rc::downgrade(&parent_rc)), text);

        parent_rc.borrow_mut().children.push(node);
    }

    fn add_tag(&mut self, tag_text: &str) {
        let (tag, mut attributes) = self.get_attributes(tag_text);

        if tag.starts_with('!') {
            return;
        }

        self.implicit_tags(Some(&tag));

        if tag.starts_with('/') {
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
        } else if SELF_CLOSING_TAGS.contains(&tag.as_str()) {
            let parent_weak: Option<Weak<RefCell<HTMLNode>>> =
                if let Some(parent_rc) = self.unfinished.last() {
                    Some(Rc::downgrade(&parent_rc.clone()))
                } else {
                    None
                };

            attributes.remove("/");

            let node = HTMLNode::new_element(parent_weak, tag, attributes);
            if let Some(parent_rc) = self.unfinished.last() {
                parent_rc.borrow_mut().children.push(node);
            }
        } else {
            let parent_weak = if let Some(parent) = self.unfinished.last() {
                Some(Rc::downgrade(&parent.clone()))
            } else {
                None
            };

            self.unfinished
                .push(HTMLNode::new_element(parent_weak, tag, attributes));
        }
    }

    fn implicit_tags(&mut self, tag: Option<&str>) {
        loop {
            let tag: &str = tag.unwrap_or("");

            let is_html_root = self
                .unfinished
                .get(0)
                .map_or(false, |n| match &n.borrow().data {
                    HTMLNodeData::Element(e) => e.tag == "html",
                    _ => false,
                });

            let is_head_second = self
                .unfinished
                .get(1)
                .map_or(false, |n| match &n.borrow().data {
                    HTMLNodeData::Element(e) => e.tag == "head",
                    _ => false,
                });

            if self.unfinished.is_empty() && tag != "html" {
                self.add_tag("html");
            } else if is_html_root
                && self.unfinished.len() == 1
                && !["head", "body", "/html"].contains(&tag)
            {
                if HEAD_TAGS.contains(&tag) {
                    self.add_tag("head");
                } else {
                    self.add_tag("body");
                }
            } else if is_html_root
                && is_head_second
                && self.unfinished.len() == 2
                && !HEAD_TAGS.contains(&tag)
                && tag != "/head"
            {
                self.add_tag("/head");
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
            let mut parts_pair = attr_pair.splitn(2, '=');
            let key = parts_pair
                .next()
                .expect("Get attributes: parts pair next failed")
                .to_lowercase();

            if let Some(mut value) = parts_pair.next() {
                if value.len() > 2 && (value.starts_with('\'') || value.starts_with('"')) {
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
