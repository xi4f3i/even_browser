use crate::dom::document::Document;
use crate::dom::element::Element;
use crate::dom::text::Text;
use std::ptr;

pub(crate) type TNodeBox = Box<TNode>;

pub(crate) type TNodePtr = ptr::NonNull<TNode>;

pub(crate) enum TNode {
    Document(Document),
    Element(Element),
    Text(Text),
}
