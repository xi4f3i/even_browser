use std::{cell::Cell, mem, ptr::NonNull};

use crate::{
    dom::node::{Node, NodeBox, NodePtr},
    html_parser::tokenizer::{Token, Tokenizer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertionMode {
    Initial,
    BeforeHtml,
}

pub(crate) struct HtmlParser {
    tokenizer: Tokenizer,
    open_elements: Vec<NodePtr>,
    document: NodeBox,
    mode: Cell<InsertionMode>,
}

impl HtmlParser {
    fn new(html: &str) -> HtmlParser {
        let mut document = Node::new_document();
        let open_elements = unsafe { vec![NonNull::new_unchecked(&mut *document)] };

        HtmlParser {
            tokenizer: Tokenizer::new(html),
            open_elements,
            document,
            mode: Cell::new(InsertionMode::Initial),
        }
    }

    fn parse(mut self) -> Box<Node> {
        loop {
            let token = self.tokenizer.next();

            match self.mode.get() {
                // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
                InsertionMode::Initial => match token {
                    // TODO: A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE - Ignore the token.
                    Token::Comment(comment) => {
                        // Insert a comment as the last child of the Document object.
                        let parent = unsafe { NonNull::new_unchecked(&mut *self.document) };
                        self.document
                            .append_child(Node::new_comment(Some(parent), &comment));
                    }
                    // TODO: A DOCTYPE token
                    _ => {
                        // If the document is not an iframe srcdoc document, then this is a parse error; if the parser cannot change the mode flag is false, set the Document to quirks mode.
                        // In any case, switch the insertion mode to "before html", then reprocess the token.
                        self.mode.set(InsertionMode::BeforeHtml);
                    }
                },
            }
        }

        todo!()
    }
}
