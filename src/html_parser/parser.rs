use std::{cell::Cell, ptr::NonNull};

use crate::{
    dom::{
        attribute::{self, Attribute},
        named_node_map::NamedNodeMap,
        node::{Node, NodeBox, NodePtr},
    },
    html_parser::tokenizer::{Tag, TagKind, Token, Tokenizer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcessResult {
    Done,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StepResult {
    Done,
    Consumed(Option<InsertionMode>),
    Ignored,
    Reprocess(InsertionMode, Token),
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

    fn parse(self) -> Box<Node> {
        loop {
            let token = self.tokenizer.next();

            match self.process_token(token) {
                ProcessResult::Done => break,
                ProcessResult::Continue => continue,
            }
        }

        self.document
    }

    fn process_token(&self, mut token: Token) -> ProcessResult {
        loop {
            match self.step(self.mode.get(), token) {
                StepResult::Done => {
                    return ProcessResult::Done;
                }
                StepResult::Consumed(mode) => {
                    if let Some(mode) = mode {
                        self.mode.set(mode);
                    }

                    return ProcessResult::Continue;
                }
                StepResult::Ignored => {
                    return ProcessResult::Continue;
                }
                StepResult::Reprocess(mode, t) => {
                    self.mode.set(mode);
                    token = t;
                }
            }
        }
    }

    fn step(&self, mode: InsertionMode, token: Token) -> StepResult {
        match mode {
            // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
            InsertionMode::Initial => match token {
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => {
                    // Ignore the token.
                    return StepResult::Ignored;
                }
                Token::Comment(c) => {
                    // Insert a comment as the last child of the Document object.
                    self.insert_comment(&c);

                    return StepResult::Consumed(None);
                }
                // TODO: A DOCTYPE token
                _ => {
                    // If the document is not an iframe srcdoc document, then this is a parse error; if the parser cannot change the mode flag is false, set the Document to quirks mode.
                    // In any case, switch the insertion mode to "before html", then reprocess the token.
                    return StepResult::Reprocess(InsertionMode::BeforeHtml, token);
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
            InsertionMode::BeforeHtml => match token {
                // TODO: A DOCTYPE token - Parse error. Ignore the token.
                Token::Comment(c) => {
                    // Insert a comment as the last child of the Document object.
                    self.insert_comment(&c);

                    return StepResult::Consumed(None);
                }
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => {
                    // Ignore the token.
                    return StepResult::Ignored;
                }
                Token::Tag(tag) => {
                    // A start tag whose tag name is "html"
                    if matches!(tag.kind, TagKind::StartTag) && tag.name == "html" {
                        // Create an element for the token in the HTML namespace, with the Document as the intended parent. Append it to the Document object. Put this element in the stack of open elements.
                        // Switch the insertion mode to "before head".
                        let element =
                            self.insert_element(&tag.name, tag.attributes, tag.self_closing);

                        return StepResult::Consumed(Some(InsertionMode::BeforeHead));
                    }

                    // An end tag whose tag name is one of: "head", "body", "html", "br"
                    if matches!(tag.kind, TagKind::EndTag)
                        && (tag.name == "head"
                            || tag.name == "body"
                            || tag.name == "html"
                            || tag.name == "br")
                    {
                        // Act as described in the "anything else" entry below.
                        let element = self.insert_element("html", Vec::new(), false);

                        return StepResult::Reprocess(InsertionMode::BeforeHead, token);
                    }

                    // Any other end tag
                    // Parse error. Ignore the token.
                    return StepResult::Ignored;
                }
                // Anything else
                _ => {
                    // Create an html element whose node document is the Document object. Append it to the Document object. Put this element in the stack of open elements.
                    // Switch the insertion mode to "before head", then reprocess the token.
                    let element = self.insert_element("html", Vec::new(), false);

                    return StepResult::Reprocess(InsertionMode::BeforeHead, token);
                } // The document element can end up being removed from the Document object, e.g. by scripts; nothing in particular happens in such cases, content continues being appended to the nodes as described in the next section.
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
            InsertionMode::BeforeHead => match token {
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => {
                    // Ignore the token.
                    return StepResult::Ignored;
                }
                Token::Comment(c) => {
                    self.insert_comment(&c);

                    return StepResult::Consumed(None);
                }
                // TODO: A DOCTYPE token - Parse error. Ignore the token.
                Token::Tag(tag) => match tag.kind {
                    TagKind::StartTag => match tag.name.as_str() {
                        // A start tag whose tag name is "html"
                        "html" => {
                            // Process the token using the rules for the "in body" insertion mode.
                            return self.step(InsertionMode::InBody, token);
                        }
                        // A start tag whose tag name is "head"
                        "head" => {
                            // Insert an HTML element for the token.
                            // Set the head element pointer to the newly created head element.
                            // Switch the insertion mode to "in head".
                            self.insert_element(&tag.name, tag.attributes, tag.self_closing);
                            return StepResult::Consumed(Some(InsertionMode::InHead));
                        }
                        _ => {
                            // Insert an HTML element for a "head" start tag token with no attributes.
                            // Set the head element pointer to the newly created head element.
                            // Switch the insertion mode to "in head".
                            // Reprocess the current token.
                            self.insert_element("head", Vec::new(), false);
                            return StepResult::Reprocess(InsertionMode::InHead, token);
                        }
                    },
                    TagKind::EndTag => match tag.name.as_str() {
                        // An end tag whose tag name is one of: "head", "body", "html", "br"
                        "head" | "body" | "html" | "br" => {
                            // Act as described in the "anything else" entry below.
                            self.insert_element("head", Vec::new(), false);
                            return StepResult::Reprocess(InsertionMode::InHead, token);
                        }
                        _ => {
                            // Parse error. Ignore the token.
                            return StepResult::Ignored;
                        }
                    },
                },
                _ => {
                    // Insert an HTML element for a "head" start tag token with no attributes.
                    // Set the head element pointer to the newly created head element.
                    // Switch the insertion mode to "in head".
                    // Reprocess the current token.
                    self.insert_element("head", Vec::new(), false);
                    return StepResult::Reprocess(InsertionMode::InHead, token);
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
            InsertionMode::InHead => match token {

            }
        }
    }

    /// When the steps below require the user agent to insert a comment while processing a comment token, optionally with an explicit insertion position position, the user agent must run the following steps:
    /// Let data be the data given in the comment token being processed.
    /// If position was specified, then let the adjusted insertion location be position. Otherwise, let adjusted insertion location be the appropriate place for inserting a node.
    /// Create a Comment node whose data attribute is set to data and whose node document is the same as that of the node in which the adjusted insertion location finds itself.
    /// Insert the newly created node at the adjusted insertion location.
    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-comment
    fn insert_comment(&mut self, comment: &str) {
        let comment = Node::new_comment(self.open_elements.last().copied(), comment);

        if let Some(parent) = self
            .open_elements
            .last_mut()
            .map(|node_ptr| unsafe { node_ptr.as_mut() })
        {
            parent.append_child(comment);
        }
    }

    /// To insert an HTML element given a token token: insert a foreign element given token, the HTML namespace, and false.
    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-an-html-element
    fn insert_element(
        &mut self,
        name: &str,
        attributes: Vec<Attribute>,
        self_closing: bool,
    ) -> NodePtr {
        let mut element = Node::new_element(
            self.open_elements.last().copied(),
            name,
            NamedNodeMap::new(attributes),
        );

        let element_ptr = element.get_ptr();

        if let Some(parent) = self
            .open_elements
            .last_mut()
            .map(|node_ptr| unsafe { node_ptr.as_mut() })
        {
            parent.append_child(element);
        }

        if !self_closing {
            self.open_elements.push(element_ptr);
        }

        element_ptr
    }
}
