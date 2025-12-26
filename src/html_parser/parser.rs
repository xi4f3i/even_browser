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
    AfterBody,
    AfterAfterBody,
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
    original_mode: Cell<InsertionMode>,
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
            original_mode: Cell::new(InsertionMode::Initial),
        }
    }

    fn parse(mut self) -> Box<Node> {
        loop {
            let token = self.tokenizer.next();

            match self.process_token(token) {
                ProcessResult::Done => break,
                ProcessResult::Continue => continue,
            }
        }

        self.document
    }

    fn process_token(&mut self, mut token: Token) -> ProcessResult {
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

    fn step(&mut self, mode: InsertionMode, token: Token) -> StepResult {
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
                        self.create_element_for_token(&tag.name, tag.attributes, tag.self_closing);

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
                        self.create_element_for_token("html", Vec::new(), false);

                        return StepResult::Reprocess(InsertionMode::BeforeHead, Token::Tag(tag));
                    }

                    // Any other end tag
                    // Parse error. Ignore the token.
                    return StepResult::Ignored;
                }
                // Anything else
                _ => {
                    // Create an html element whose node document is the Document object. Append it to the Document object. Put this element in the stack of open elements.
                    // Switch the insertion mode to "before head", then reprocess the token.
                    self.create_element_for_token("html", Vec::new(), false);

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
                            return self.step(InsertionMode::InBody, Token::Tag(tag));
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
                            return StepResult::Reprocess(InsertionMode::InHead, Token::Tag(tag));
                        }
                    },
                    TagKind::EndTag => match tag.name.as_str() {
                        // An end tag whose tag name is one of: "head", "body", "html", "br"
                        "head" | "body" | "html" | "br" => {
                            // Act as described in the "anything else" entry below.
                            self.insert_html_element("head", Vec::new(), false);
                            return StepResult::Reprocess(InsertionMode::InHead, Token::Tag(tag));
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
                            return self.step(InsertionMode::InBody, Token::Tag(tag));
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
                            self.original_mode.set(InsertionMode::InHead);
                            return StepResult::Consumed(Some(InsertionMode::Text));
                        }
                        // A start tag whose tag name is "noscript", if the scripting flag is enabled
                        // A start tag whose tag name is one of: "noframes", "style"
                        "noscript" | "noframes" | "style" => {
                            // Follow the generic raw text element parsing algorithm.
                            self.insert_html_element(&tag.name, tag.attributes, false);
                            // TODO: https://html.spec.whatwg.org/multipage/parsing.html#generic-raw-text-element-parsing-algorithm
                            self.original_mode.set(InsertionMode::InHead);
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
                            return StepResult::Reprocess(InsertionMode::AfterHead, Token::Tag(tag));
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
                            return StepResult::Reprocess(InsertionMode::AfterHead, Token::Tag(tag));
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
                            return self.step(InsertionMode::InBody, Token::Tag(tag));
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
                            return StepResult::Reprocess(InsertionMode::InBody, Token::Tag(tag));
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
                            return StepResult::Reprocess(InsertionMode::InBody, Token::Tag(tag));
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
                // TODO: A character token that is U+0000 NULL - Parse error. Ignore the token.
                Token::Character(c) => {
                    if c == '\t' || c == '\n' || c == '\x0C' || c == '\r' || c == ' ' {
                        // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                        // Reconstruct the active formatting elements, if any.
                        // Insert the token's character.
                    } else {
                        // Any other character token
                        // Reconstruct the active formatting elements, if any.
                        // Insert the token's character.
                        // TODO: Set the frameset-ok flag to "not ok".
                    }
                    self.insert_character(c);

                    return StepResult::Consumed(None);
                }
                // A comment token
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
                            // TODO: Parse error. If there is a template element on the stack of open elements, then ignore the token. Otherwise, for each attribute on the token, check to see if the attribute is already present on the top element of the stack of open elements. If it is not, add the attribute and its corresponding value to that element.
                            return StepResult::Ignored;
                        }
                        // A start tag whose tag name is one of: "base", "basefont", "bgsound", "link", "meta", "noframes", "script", "style", "template", "title"
                        "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes"
                        | "script" | "style" | "template" | "title" => {
                            // Process the token using the rules for the "in head" insertion mode.
                            return self.step(InsertionMode::InHead, Token::Tag(tag));
                        }
                        // A start tag whose tag name is "body"
                        "body" => {
                            // TODO: Parse error. If the stack of open elements has only one node on it, or if the second element on the stack of open elements is not a body element, or if there is a template element on the stack of open elements, then ignore the token. (fragment case or there is a template element on the stack) Otherwise, set the frameset-ok flag to "not ok"; then, for each attribute on the token, check to see if the attribute is already present on the body element (the second element) on the stack of open elements, and if it is not, add the attribute and its corresponding value to that element.
                            return StepResult::Ignored;
                        }
                        // TODO: A start tag whose tag name is "frameset"
                        // TODO: A start tag whose tag name is one of: "address", "article", "aside", "blockquote", "center", "details", "dialog", "dir", "div", "dl", "fieldset", "figcaption", "figure", "footer", "header", "hgroup", "main", "menu", "nav", "ol", "p", "search", "section", "summary", "ul"
                        // TODO: A start tag whose tag name is one of: "h1", "h2", "h3", "h4", "h5", "h6"
                        // TODO: A start tag whose tag name is one of: "pre", "listing"
                        // TODO: A start tag whose tag name is "form"
                        // TODO: A start tag whose tag name is "li"
                        // TODO: A start tag whose tag name is one of: "dd", "dt"
                        // TODO: A start tag whose tag name is "plaintext"
                        // TODO: A start tag whose tag name is "button"
                        // TODO: A start tag whose tag name is "a"
                        // TODO: A start tag whose tag name is one of: "b", "big", "code", "em", "font", "i", "s", "small", "strike", "strong", "tt", "u"
                        // TODO: A start tag whose tag name is "nobr"
                        // TODO: A start tag whose tag name is one of: "applet", "marquee", "object"
                        // TODO: A start tag whose tag name is "table"
                        // TODO: A start tag whose tag name is one of: "area", "br", "embed", "img", "keygen", "wbr"
                        // TODO: A start tag whose tag name is "input"
                        // TODO: A start tag whose tag name is one of: "param", "source", "track"
                        // TODO: A start tag whose tag name is "hr"
                        // TODO: A start tag whose tag name is "image"
                        // TODO: A start tag whose tag name is "textarea"
                        // TODO: A start tag whose tag name is "xmp"
                        // TODO: A start tag whose tag name is "iframe"
                        // TODO: A start tag whose tag name is "noembed"
                        // TODO: A start tag whose tag name is "noscrA start tag whose tag name is "noscript", if the scripting flag is enabledipt", if the scripting flag is enabled
                        // TODO: A start tag whose tag name is "select"
                        // TODO: A start tag whose tag name is "option"
                        // TODO: A start tag whose tag name is "optgroup"
                        // TODO: A start tag whose tag name is one of: "rb", "rtc"
                        // TODO: A start tag whose tag name is one of: "rp", "rt"
                        // TODO: A start tag whose tag name is "math"
                        // TODO: A start tag whose tag name is "svg"
                        // TODO: A start tag whose tag name is one of: "caption", "col", "colgroup", "frame", "head", "tbody", "td", "tfoot", "th", "thead", "tr"
                        // Any other start tag
                        _ => {
                            // TODO: Reconstruct the active formatting elements, if any.
                            // Insert an HTML element for the token.
                            // This element will be an ordinary element. With one exception: if the scripting flag is disabled, it can also be a noscript element.
                            self.insert_html_element(&tag.name, tag.attributes, tag.self_closing);
                            return StepResult::Consumed(None);
                        }
                    },
                    TagKind::EndTag => match tag.name.as_str() {
                        // TODO: An end tag whose tag name is "template" - Process the token using the rules for the "in head" insertion mode.
                        // An end tag whose tag name is "body"
                        "body" => {
                            // TODO: If the stack of open elements does not have a body element in scope, this is a parse error; ignore the token.
                            // TODO: Otherwise, if there is a node in the stack of open elements that is not either a dd element, a dt element, an li element, an optgroup element, an option element, a p element, an rb element, an rp element, an rt element, an rtc element, a tbody element, a td element, a tfoot element, a th element, a thead element, a tr element, the body element, or the html element, then this is a parse error.
                            // Switch the insertion mode to "after body".
                            return StepResult::Consumed(Some(InsertionMode::AfterBody));
                        }
                        // An end tag whose tag name is "html"
                        "html" => {
                            // TODO: If the stack of open elements does not have a body element in scope, this is a parse error; ignore the token.
                            // TODO: Otherwise, if there is a node in the stack of open elements that is not either a dd element, a dt element, an li element, an optgroup element, an option element, a p element, an rb element, an rp element, an rt element, an rtc element, a tbody element, a td element, a tfoot element, a th element, a thead element, a tr element, the body element, or the html element, then this is a parse error.
                            // Switch the insertion mode to "after body".
                            // Reprocess the token.
                            return StepResult::Reprocess(InsertionMode::AfterBody, Token::Tag(tag));
                        }
                        // TODO: An end tag whose tag name is one of: "address", "article", "aside", "blockquote", "button", "center", "details", "dialog", "dir", "div", "dl", "fieldset", "figcaption", "figure", "footer", "header", "hgroup", "listing", "main", "menu", "nav", "ol", "pre", "search", "section", "select", "summary", "ul"
                        // TODO: An end tag whose tag name is "form"
                        // TODO: An end tag whose tag name is "p"
                        // TODO: An end tag whose tag name is "li"
                        // TODO: An end tag whose tag name is one of: "dd", "dt"
                        // TODO: An end tag whose tag name is one of: "h1", "h2", "h3", "h4", "h5", "h6"
                        // TODO: An end tag whose tag name is "sarcasm"
                        // TODO: An end tag whose tag name is one of: "a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt", "u"
                        // TODO: An end tag token whose tag name is one of: "applet", "marquee", "object"
                        // TODO: An end tag whose tag name is "br"
                        // Any other end tag
                        _ => {
                            // TODO: Run these steps:
                            //     Initialize node to be the current node (the bottommost node of the stack).
                            //     Loop: If node is an HTML element with the same tag name as the token, then:
                            //     Generate implied end tags, except for HTML elements with the same tag name as the token.
                            //     If node is not the current node, then this is a parse error.
                            //     Pop all the nodes from the current node up to node, including node, then stop these steps.
                            //     Otherwise, if node is in the special category, then this is a parse error; ignore the token, and return.
                            //     Set node to the previous entry in the stack of open elements.
                            //     Return to the step labeled loop.
                            let current_node = self.current_node();
                            if let NodeSubtype::Element(e) = current_node.subtype()
                                && e.tag_name() == tag.name
                            {
                                self.open_elements.pop();
                            }

                            return StepResult::Consumed(None);
                        }
                    },
                },
                // An end-of-file token
                Token::EOF => {
                    // TODO: If the stack of template insertion modes is not empty, then process the token using the rules for the "in template" insertion mode. Otherwise, follow these steps: If there is a node in the stack of open elements that is not either a dd element, a dt element, an li element, an optgroup element, an option element, a p element, an rb element, an rp element, an rt element, an rtc element, a tbody element, a td element, a tfoot element, a th element, a thead element, a tr element, the body element, or the html element, then this is a parse error. Stop parsing.
                    return StepResult::Done;
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incdata
            InsertionMode::Text => match token {
                // A character token
                Token::Character(c) => {
                    // Insert the token's character.
                    // This can never be a U+0000 NULL character; the tokenizer converts those to U+FFFD REPLACEMENT CHARACTER characters.
                    self.insert_character(c);
                    return StepResult::Consumed(None);
                }
                // An end-of-file token
                Token::EOF => {
                    // TODO: Parse error.
                    // TODO: If the current node is a script element, then set its already started to true.
                    // Pop the current node off the stack of open elements.
                    // Switch the insertion mode to the original insertion mode and reprocess the token.
                    self.open_elements.pop();
                    return StepResult::Reprocess(self.original_mode.get(), token);
                }
                Token::Tag(tag) => match &tag.kind {
                    TagKind::StartTag => {
                        self.open_elements.pop();
                        return StepResult::Reprocess(self.original_mode.get(), Token::Tag(tag));
                    }
                    TagKind::EndTag => match tag.name.as_str() {
                        // TODO: An end tag whose tag name is "script"
                        // Any other end tag
                        _ => {
                            // Pop the current node off the stack of open elements.
                            // Switch the insertion mode to the original insertion mode.
                            self.open_elements.pop();
                            return StepResult::Reprocess(self.original_mode.get(), Token::Tag(tag));
                        }
                    },
                },
                _ => {
                    self.open_elements.pop();
                    return StepResult::Reprocess(self.original_mode.get(), token);
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
            InsertionMode::AfterBody => match token {
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => {
                    // Process the token using the rules for the "in body" insertion mode.
                    return self.step(InsertionMode::InBody, token);
                }
                // A comment token
                Token::Comment(c) => {
                    // Insert a comment as the last child of the first element in the stack of open elements (the html element).
                    self.insert_comment(&c);
                    return StepResult::Consumed(None);
                }
                // TODO: A DOCTYPE token - Parse error. Ignore the token.
                Token::Tag(tag) => match &tag.kind {
                    TagKind::StartTag => match tag.name.as_str() {
                        // A start tag whose tag name is "html"
                        "html" => {
                            // Process the token using the rules for the "in body" insertion mode.
                            return self.step(InsertionMode::InBody, Token::Tag(tag));
                        }
                        _ => {
                            return StepResult::Reprocess(InsertionMode::InBody, Token::Tag(tag));
                        }
                    },
                    TagKind::EndTag => match tag.name.as_str() {
                        // An end tag whose tag name is "html"
                        "html" => {
                            // TODO: If the parser was created as part of the HTML fragment parsing algorithm, this is a parse error; ignore the token. (fragment case)
                            // Otherwise, switch the insertion mode to "after after body".
                            return StepResult::Consumed(Some(InsertionMode::AfterAfterBody));
                        }
                        _ => {
                            return StepResult::Reprocess(InsertionMode::InBody, Token::Tag(tag));
                        }
                    },
                },
                // An end-of-file token
                Token::EOF => {
                    // Stop parsing.
                    return StepResult::Done;
                }
                // Anything else
                _ => {
                    // Parse error. Switch the insertion mode to "in body" and reprocess the token.
                    return StepResult::Reprocess(InsertionMode::InBody, token);
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
            InsertionMode::AfterAfterBody => match token {
                // A comment token
                Token::Comment(c) => {
                    // Insert a comment as the last child of the Document object.
                    self.insert_comment_into_document(&c);
                    return StepResult::Consumed(None);
                }
                // TODO: A DOCTYPE token
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => {
                    // Process the token using the rules for the "in body" insertion mode.
                    return self.step(InsertionMode::InBody, token);
                }
                // A start tag whose tag name is "html"
                Token::Tag(tag) if matches!(tag.kind, TagKind::StartTag) && tag.name == "html" => {
                    // Process the token using the rules for the "in body" insertion mode.
                    return self.step(InsertionMode::InBody, Token::Tag(tag));
                }
                // An end-of-file token
                Token::EOF => {
                    // Stop parsing.
                    return StepResult::Done;
                }
                // Anything else
                _ => {
                    // Parse error. Switch the insertion mode to "in body" and reprocess the token.
                    return StepResult::Reprocess(InsertionMode::InBody, token);
                }
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
