use crate::dom::document::Document;
use crate::dom::element::Element;
use crate::dom::text::Text;
use std::ptr;

pub(crate) type TNodePtr = ptr::NonNull<TNode>;

pub(crate) enum TNode {
    Document(Document),
    Element(Element),
    Text(Text),
}

impl TNode {
    pub(crate) fn append_child(&self, node: TNodePtr) {
        match self {
            TNode::Document(doc) => doc.append_child(node),
            TNode::Element(elem) => elem.append_child(node),
            TNode::Text(text) => {
                println!(
                    "[TNode] Error: can't append a node to a Text({})",
                    text.data.borrow()
                );
            }
        }
    }

    pub(crate) fn insert_character(&self, ch: char, self_ptr: TNodePtr) {
        match self {
            TNode::Document(_) => {
                // If the adjusted insertion location is in a Document node, then return.
                // The DOM will not let Document nodes have Text node children, so they are dropped on the floor.
            }
            TNode::Element(elem) => elem.insert_character(ch, self_ptr),
            TNode::Text(text) => {
                println!(
                    "[TNode] Error: can't insert a char({}) to a Text({})",
                    ch,
                    text.data.borrow()
                );
            }
        }
    }
}
