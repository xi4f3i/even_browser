use crate::html::constant::{HEAD_TAGS, SELF_CLOSING_ELEMENTS};
use crate::html::node::NodeID;
use crate::html::tree::DOMTree;
use std::collections::HashMap;

pub fn parse_html(html: &str) -> DOMTree {
    let mut tree = DOMTree::new();

    let chars = html.chars().collect::<Vec<char>>();
    let mut unfinished: Vec<NodeID> = Vec::new();

    let root = parse(&mut tree, &chars, &mut unfinished);

    tree.root = Some(root);

    tree
}

fn parse(tree: &mut DOMTree, chars: &Vec<char>, unfinished: &mut Vec<NodeID>) -> NodeID {
    let mut start: usize = 0;
    let mut idx: usize = 0;
    let mut in_tag = false;

    while idx < chars.len() {
        match chars[idx] {
            '<' => {
                in_tag = true;
                if idx > start {
                    let text = chars[start..idx].iter().collect::<String>();
                    add_text(tree, unfinished, text);
                }

                idx += 1;
                start = idx;
            }
            '>' => {
                in_tag = false;

                let text = chars[start..idx].iter().collect::<String>();
                add_element_or_comment(tree, unfinished, text);

                idx += 1;
                start = idx;
            }
            _ => idx += 1,
        }
    }

    if !in_tag && start < chars.len() {
        add_text(
            tree,
            unfinished,
            chars[start..chars.len()].iter().collect::<String>(),
        );
    }

    finish(tree, unfinished)
}

fn finish(tree: &mut DOMTree, unfinished: &mut Vec<NodeID>) -> NodeID {
    if unfinished.is_empty() {
        implicit_elements(tree, unfinished, "");
    }

    while unfinished.len() > 1 {
        unfinished.pop();
    }

    unfinished.pop().expect("No root found")
}

fn add_text(tree: &mut DOMTree, unfinished: &mut Vec<NodeID>, text: String) {
    if text.trim().is_empty() {
        return;
    }

    implicit_elements(tree, unfinished, "");

    let parent_id = unfinished.last().copied();
    tree.add_text(parent_id, text);
}

fn add_element_or_comment(tree: &mut DOMTree, unfinished: &mut Vec<NodeID>, text: String) {
    if text.starts_with("!--") && text.ends_with("--") && text.len() >= 5 {
        add_comment(tree, unfinished, text);
    } else {
        add_element(tree, unfinished, text);
    }
}

fn add_element(tree: &mut DOMTree, unfinished: &mut Vec<NodeID>, text: String) {
    if text.starts_with('!') {
        return;
    }

    let is_self_closing = text.ends_with('/');

    let (tag, attributes) = get_tag_and_attributes(if is_self_closing {
        text.trim_end_matches('/')
    } else {
        &text
    });
    implicit_elements(tree, unfinished, &tag);

    if tag.starts_with("/") {
        if unfinished.len() == 1 {
            return;
        }
        unfinished.pop();
    } else if SELF_CLOSING_ELEMENTS.contains(&tag.as_str()) || is_self_closing {
        let parent_id = unfinished.last().copied();
        tree.add_element(parent_id, tag, true, attributes);
    } else {
        let parent_id = unfinished.last().copied();
        let id = tree.add_element(parent_id, tag, false, attributes);
        unfinished.push(id);
    }
}

fn implicit_elements(tree: &mut DOMTree, unfinished: &mut Vec<NodeID>, tag: &str) {
    loop {
        let open_tags: Vec<&str> = unfinished
            .iter()
            .map(|id| tree.get_node(*id))
            .flatten()
            .map(|node| node.get_tag())
            .flatten()
            .collect();

        if open_tags.is_empty() && tag != "html" {
            add_element(tree, unfinished, "html".to_string());
        } else if open_tags.len() == 1
            && open_tags[0] == "html"
            && !["head", "body", "/html"].contains(&tag)
        {
            if HEAD_TAGS.contains(&tag) {
                add_element(tree, unfinished, "head".to_string());
            } else {
                add_element(tree, unfinished, "body".to_string());
            }
        } else if open_tags.len() == 2
            && open_tags[0] == "html"
            && open_tags[1] == "head"
            && !HEAD_TAGS.contains(&tag)
            && tag != "/head"
        {
            add_element(tree, unfinished, "/head".to_string());
        } else {
            break;
        }
    }
}

fn get_tag_and_attributes(text: &str) -> (String, HashMap<String, String>) {
    let mut attributes = HashMap::new();

    let chars = text.chars().collect::<Vec<char>>();

    let mut idx: usize = 0;

    let tag = get_tag(&chars, &mut idx);

    while idx < chars.len() {
        whitespace(&chars, &mut idx);
        let name = name(&chars, &mut idx);
        if name.is_empty() {
            continue;
        }
        if idx >= chars.len() {
            attributes.insert(name.to_lowercase(), "".to_string());
            break;
        }
        let value = value(&chars, &mut idx);
        attributes.insert(name.to_lowercase(), value);
    }

    (tag.to_lowercase(), attributes)
}

fn value(chars: &Vec<char>, idx: &mut usize) -> String {
    let end_char: char = match chars[*idx] {
        '\'' => {
            *idx += 1;
            '\''
        }
        '"' => {
            *idx += 1;
            '"'
        }
        _ => ' ',
    };

    let tmp = *idx;

    while *idx < chars.len() {
        if chars[*idx] == end_char {
            *idx += 1;
            return chars[tmp..*idx - 1].iter().collect::<String>();
        }

        *idx += 1;
    }

    chars[tmp..*idx].iter().collect::<String>()
}

fn name(chars: &Vec<char>, idx: &mut usize) -> String {
    let tmp = *idx;

    while *idx < chars.len() {
        if chars[*idx] == '=' {
            *idx += 1;
            return chars[tmp..*idx - 1].iter().collect::<String>();
        }

        *idx += 1;
    }

    chars[tmp..*idx].iter().collect::<String>()
}

fn whitespace(chars: &Vec<char>, idx: &mut usize) {
    while *idx < chars.len() {
        if !chars[*idx].is_whitespace() {
            break;
        }

        *idx += 1;
    }
}

fn get_tag(chars: &Vec<char>, idx: &mut usize) -> String {
    let tmp = *idx;

    while *idx < chars.len() {
        if chars[*idx].is_whitespace() {
            *idx += 1;
            return chars[tmp..*idx - 1].iter().collect::<String>();
        }

        *idx += 1;
    }

    chars[tmp..*idx].iter().collect::<String>()
}

fn add_comment(tree: &mut DOMTree, unfinished: &mut Vec<NodeID>, text: String) {
    let comment = text
        .trim_start_matches("!--")
        .trim_end_matches("--")
        .to_string();

    let parent_id = unfinished.last().copied();
    tree.add_comment(parent_id, comment);
}
