use crate::{
    dom::node::{Node, NodeBox, NodePtr},
    html_parser_backup::tokenizer::Tokenizer,
};

/// https://html.spec.whatwg.org/multipage/parsing.html
#[derive(Debug)]
pub(crate) struct HtmlParser {
    tokenizer: Tokenizer,
    doc: NodeBox,
    open_elements: Vec<NodePtr>,
}

impl HtmlParser {
    pub(crate) fn new(html: &str) -> HtmlParser {
        HtmlParser {
            tokenizer: Tokenizer::new(html),
            doc: Node::new_document(),
            open_elements: Vec::new(),
        }
    }
}
