use std::{cell::Cell, ptr::NonNull};

use crate::{
    dom::{
        attribute::{self, Attribute},
        named_node_map::NamedNodeMap,
        node::{Node, NodeBox, NodePtr, NodeSubtype},
    },
    html_parser::tokenizer::{Tag, TagKind, Token, Tokenizer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    Text,
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
                    self.insert_comment_into_document(&c);

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
                    self.insert_comment_into_document(&c);

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
                        let element = self.create_element_for_token(
                            &tag.name,
                            tag.attributes,
                            tag.self_closing,
                        );

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
                        let element = self.create_element_for_token("html", Vec::new(), false);

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
                    let element = self.create_element_for_token("html", Vec::new(), false);

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
                    // Insert a comment.
                    self.insert_comment(&c);

                    return StepResult::Consumed(None);
                }
                // TODO: A DOCTYPE token - Parse error. Ignore the token.
                Token::Tag(tag) => match &tag.kind {
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
                            self.insert_html_element(&tag.name, tag.attributes, tag.self_closing);
                            return StepResult::Consumed(Some(InsertionMode::InHead));
                        }
                        _ => {
                            // Insert an HTML element for a "head" start tag token with no attributes.
                            // Set the head element pointer to the newly created head element.
                            // Switch the insertion mode to "in head".
                            // Reprocess the current token.
                            self.insert_html_element("head", Vec::new(), false);
                            return StepResult::Reprocess(InsertionMode::InHead, token);
                        }
                    },
                    TagKind::EndTag => match tag.name.as_str() {
                        // An end tag whose tag name is one of: "head", "body", "html", "br"
                        "head" | "body" | "html" | "br" => {
                            // Act as described in the "anything else" entry below.
                            self.insert_html_element("head", Vec::new(), false);
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
                    self.insert_html_element("head", Vec::new(), false);
                    return StepResult::Reprocess(InsertionMode::InHead, token);
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
            InsertionMode::InHead => match token {
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => {
                    // Insert the character.
                    if let Token::Character(c) = token {
                        self.insert_character(c);
                    }
                    return StepResult::Consumed(None);
                }
                Token::Comment(c) => {
                    // Insert a comment.
                    self.insert_comment(&c);

                    return StepResult::Consumed(None);
                }
                // TODO: A DOCTYPE token - Parse error. Ignore the token.
                Token::Tag(tag) => match &tag.kind {
                    TagKind::StartTag => match tag.name.as_str() {
                        // A start tag whose tag name is "html"
                        "html" => {
                            // Process the token using the rules for the "in body" insertion mode.
                            return self.step(InsertionMode::InBody, token);
                        }
                        // A start tag whose tag name is one of: "base", "basefont", "bgsound", "link"
                        "base" | "basefont" | "bgsound" | "link" => {
                            // Insert an HTML element for the token. Immediately pop the current node off the stack of open elements.
                            // Acknowledge the token's self-closing flag, if it is set.
                            self.insert_html_element(&tag.name, tag.attributes, true);
                            return StepResult::Consumed(None);
                        }
                        // A start tag whose tag name is "meta"
                        "meta" => {
                            // Insert an HTML element for the token. Immediately pop the current node off the stack of open elements.
                            // Acknowledge the token's self-closing flag, if it is set.
                            self.insert_html_element(&tag.name, tag.attributes, true);
                            return StepResult::Consumed(None);
                            // TODO: If the active speculative HTML parser is null, then: If the element has a charset attribute, and getting an encoding from its value results in an encoding, and the confidence is currently tentative, then change the encoding to the resulting encoding. Otherwise, if the element has an http-equiv attribute whose value is an ASCII case-insensitive match for the string "Content-Type", and the element has a content attribute, and applying the algorithm for extracting a character encoding from a meta element to that attribute's value returns an encoding, and the confidence is currently tentative, then change the encoding to the extracted encoding. The speculative HTML parser doesn't speculatively apply character encoding declarations in order to reduce implementation complexity.
                        }
                        // A start tag whose tag name is "title"
                        "title" => {
                            // Follow the generic RCDATA element parsing algorithm.
                            self.insert_html_element(&tag.name, tag.attributes, false);
                            // TODO: https://html.spec.whatwg.org/multipage/parsing.html#generic-rcdata-element-parsing-algorithm
                            return StepResult::Consumed(Some(InsertionMode::Text));
                        }
                        // A start tag whose tag name is "noscript", if the scripting flag is enabled
                        // A start tag whose tag name is one of: "noframes", "style"
                        "noscript" | "noframes" | "style" => {
                            // Follow the generic raw text element parsing algorithm.
                            self.insert_html_element(&tag.name, tag.attributes, false);
                            // TODO: https://html.spec.whatwg.org/multipage/parsing.html#generic-raw-text-element-parsing-algorithm
                            return StepResult::Consumed(Some(InsertionMode::Text));
                        }
                        // TODO: A start tag whose tag name is "noscript", if the scripting flag is disabled
                        // TODO: A start tag whose tag name is "script"
                        // TODO: A start tag whose tag name is "template"
                        // A start tag whose tag name is "head"
                        "head" => {
                            // Parse error. Ignore the token.
                            return StepResult::Ignored;
                        }
                        _ => {
                            // Pop the current node (which will be the head element) off the stack of open elements.
                            // Switch the insertion mode to "after head".
                            // Reprocess the token.
                            self.open_elements.pop();
                            return StepResult::Reprocess(InsertionMode::AfterHead, token);
                        }
                    },
                    TagKind::EndTag => match tag.name.as_str() {
                        // An end tag whose tag name is "head"
                        "head" => {
                            // Pop the current node (which will be the head element) off the stack of open elements.
                            // Switch the insertion mode to "after head".
                            self.open_elements.pop();
                            return StepResult::Consumed(Some(InsertionMode::AfterHead));
                        }
                        // An end tag whose tag name is one of: "body", "html", "br"
                        "body" | "html" | "br" => {
                            // Act as described in the "anything else" entry below.
                            self.open_elements.pop();
                            return StepResult::Reprocess(InsertionMode::AfterHead, token);
                        }
                        // TODO: An end tag whose tag name is "template"
                        // Any other end tag
                        _ => {
                            // Parse error. Ignore the token.
                            return StepResult::Ignored;
                        }
                    },
                },
                _ => {
                    // Pop the current node (which will be the head element) off the stack of open elements.
                    // Switch the insertion mode to "after head".
                    // Reprocess the token.
                    self.open_elements.pop();
                    return StepResult::Reprocess(InsertionMode::AfterHead, token);
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#the-after-head-insertion-mode
            InsertionMode::AfterHead => match token {
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => {
                    // Insert the character.
                    if let Token::Character(c) = token {
                        self.insert_character(c);
                    }
                    return StepResult::Consumed(None);
                }
                Token::Comment(c) => {
                    // Insert a comment.
                    self.insert_comment(&c);

                    return StepResult::Consumed(None);
                }
                // TODO: A DOCTYPE token - Parse error. Ignore the token.
                Token::Tag(tag) => match &tag.kind {
                    TagKind::StartTag => match tag.name.as_str() {
                        // A start tag whose tag name is "html"
                        "html" => {
                            // Process the token using the rules for the "in body" insertion mode.
                            return self.step(InsertionMode::InBody, token);
                        }
                        // A start tag whose tag name is "body"
                        "body" => {
                            // Insert an HTML element for the token.
                            // TODO: Set the frameset-ok flag to "not ok".
                            // Switch the insertion mode to "in body".
                            self.insert_html_element(&tag.name, tag.attributes, tag.self_closing);
                            return StepResult::Consumed(Some(InsertionMode::InBody));
                        }
                        // TODO: A start tag whose tag name is "frameset"
                        // TODO: A start tag whose tag name is one of: "base", "basefont", "bgsound", "link", "meta", "noframes", "script", "style", "template", "title"
                        // A start tag whose tag name is "head"
                        "head" => {
                            // Parse error. Ignore the token.
                            return StepResult::Ignored;
                        }
                        _ => {
                            // Insert an HTML element for a "body" start tag token with no attributes.
                            // Switch the insertion mode to "in body".
                            // Reprocess the current token.
                            self.insert_html_element("body", vec![], false);
                            return StepResult::Reprocess(InsertionMode::InBody, token);
                        }
                    },
                    TagKind::EndTag => match tag.name.as_str() {
                        // TODO: An end tag whose tag name is "template"
                        // An end tag whose tag name is one of: "body", "html", "br"
                        "body" | "html" | "br" => {
                            // Act as described in the "anything else" entry below.
                            // Insert an HTML element for a "body" start tag token with no attributes.
                            // Switch the insertion mode to "in body".
                            // Reprocess the current token.
                            self.insert_html_element("body", vec![], false);
                            return StepResult::Reprocess(InsertionMode::InBody, token);
                        }
                        // Any other end tag
                        _ => {
                            // Parse error. Ignore the token.
                            return StepResult::Ignored;
                        }
                    },
                },
                // Anything else
                _ => {
                    // Insert an HTML element for a "body" start tag token with no attributes.
                    // Switch the insertion mode to "in body".
                    // Reprocess the current token.
                    self.insert_html_element("body", vec![], false);
                    return StepResult::Reprocess(InsertionMode::InBody, token);
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody
            InsertionMode::InBody => match token {
                
            },
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-an-html-element
    fn insert_html_element(
        &mut self,
        name: &str,
        attributes: Vec<Attribute>,
        self_closing: bool,
    ) -> NodePtr {
        let insertion_location = self.appropriate_place_for_insertion();
        let mut element = Node::new_element(
            Some(insertion_location.get_ptr()),
            name,
            NamedNodeMap::new(attributes),
        );

        let element_ptr = element.get_ptr();

        insertion_location.append_child(element);

        if !self_closing {
            self.open_elements.push(element_ptr);
        }

        element_ptr
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-character
    fn insert_character(&mut self, data: char) {
        let insertion_location = self.appropriate_place_for_insertion();

        if let NodeSubtype::Document(_) = insertion_location.subtype() {
            // If the adjusted insertion location is in a Document node, then return.
            // The DOM will not let Document nodes have Text node children, so they are dropped on the floor.
            return;
        }

        // is the last child of insertion_location a Text node?
        if let Some(last_child) = insertion_location.last_child_mut()
            && let NodeSubtype::Text(t) = last_child.subtype_mut()
        {
            t.push(data);
        } else {
            let text = Node::new_text(Some(insertion_location.get_ptr()), &data.to_string());
            insertion_location.append_child(text);
        }
    }

    /// https://dom.spec.whatwg.org/#concept-create-element
    fn create_element(&mut self) -> NodePtr {
        todo!();
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#create-an-element-for-the-token
    fn create_element_for_token(
        &mut self,
        name: &str,
        attributes: Vec<Attribute>,
        self_closing: bool,
    ) -> NodePtr {
        let insertion_location = self.appropriate_place_for_insertion();
        let mut element = Node::new_element(
            Some(insertion_location.get_ptr()),
            name,
            NamedNodeMap::new(attributes),
        );

        let element_ptr = element.get_ptr();

        insertion_location.append_child(element);

        if !self_closing {
            self.open_elements.push(element_ptr);
        }

        element_ptr
    }

    fn insert_comment_into_document(&mut self, data: &str) {
        let insertion_location = self.document.as_mut();
        let comment = Node::new_comment(Some(insertion_location.get_ptr()), data);
        insertion_location.append_child(comment);
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-comment
    fn insert_comment(&mut self, data: &str) {
        let insertion_location = self.appropriate_place_for_insertion();
        let parent = insertion_location.get_ptr();
        let comment = Node::new_comment(Some(parent), data);
        insertion_location.append_child(comment);
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#appropriate-place-for-inserting-a-node
    fn appropriate_place_for_insertion(&mut self) -> &mut Node {
        // TODO: override target, foster parenting and template element
        self.current_node()
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#current-node
    fn current_node(&mut self) -> &mut Node {
        // The current node is the bottommost node in this stack of open elements.
        self.open_elements
            .last_mut()
            .map(|node_ptr| unsafe { node_ptr.as_mut() })
            .expect("current_node is invalid.")
    }
}
