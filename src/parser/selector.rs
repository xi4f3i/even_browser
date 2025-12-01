use crate::parser::css_parser::CSSRule;
use crate::parser::html_node::{HTMLNodeData, HTMLNodeRef};

pub type Priority = usize;

#[derive(Debug, Clone)]
pub struct TagSelector {
    tag: String,
    priority: Priority,
}

impl TagSelector {
    pub fn new(tag: String) -> Self {
        Self { tag, priority: 1 }
    }

    pub fn matches(&self, node: HTMLNodeRef) -> bool {
        match &node.borrow().data {
            HTMLNodeData::Element(e) => e.tag == self.tag,
            HTMLNodeData::Text(_) => false,
        }
    }

    fn get_priority(&self) -> Priority {
        self.priority
    }
}

#[derive(Debug, Clone)]
pub struct DescendantSelector {
    ancestor: Box<Selector>,
    descendant: Box<Selector>,
    priority: Priority,
}

impl DescendantSelector {
    pub fn new(ancestor: Selector, descendant: Selector) -> Self {
        let priority = ancestor.get_priority() + descendant.get_priority();
        Self {
            ancestor: Box::new(ancestor),
            descendant: Box::new(descendant),
            priority,
        }
    }

    pub fn matches(&self, node: HTMLNodeRef) -> bool {
        if !self.descendant.matches(node.clone()) {
            return false;
        }

        let mut tmp = node;

        loop {
            let parent_rc = {
                let Some(parent_weak) = &tmp.borrow().parent else {
                    break;
                };
                parent_weak.upgrade()
            };

            if let Some(parent_rc) = parent_rc {
                if self.ancestor.matches(parent_rc.clone()) {
                    return true;
                }

                tmp = parent_rc;
            } else {
                break;
            }
        }

        false
    }

    fn get_priority(&self) -> Priority {
        self.priority
    }
}

#[derive(Debug, Clone)]
pub enum Selector {
    Tag(TagSelector),
    Descendant(DescendantSelector),
}

impl Selector {
    pub fn new_tag(tag: String) -> Self {
        Self::Tag(TagSelector::new(tag))
    }

    pub fn new_descendant(ancestor: Selector, descendant: Selector) -> Self {
        Self::Descendant(DescendantSelector::new(ancestor, descendant))
    }

    pub fn matches(&self, node: HTMLNodeRef) -> bool {
        match self {
            Selector::Tag(selector) => selector.matches(node),
            Selector::Descendant(selector) => selector.matches(node),
        }
    }

    pub fn get_priority(&self) -> Priority {
        match self {
            Selector::Tag(selector) => selector.get_priority(),
            Selector::Descendant(selector) => selector.get_priority(),
        }
    }
}

pub fn cascade_priority(rule: &CSSRule) -> Priority {
    rule.0.get_priority()
}
