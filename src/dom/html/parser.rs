use crate::dom::attr::Attr;
use crate::dom::document::Document;
use crate::dom::element::Element;
use crate::dom::html::mode::InsertionMode;
use crate::dom::html::node::{TNode, TNodePtr};
use crate::dom::html::token::{Tag, Token};
use crate::dom::html::tokenizer::Tokenizer;
use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

enum StepResult {
    Done,
    Continue,
}

enum ProcessResult {
    Done,
    Consumed(Option<InsertionMode>),
    Ignored,
    Reprocess(InsertionMode),
}

struct HtmlParser {
    tokenizer: Tokenizer,
    open_elements: RefCell<Vec<TNodePtr>>,
    mode: Cell<InsertionMode>,
    prev_mode: Cell<InsertionMode>,
    document: Cell<TNodePtr>,
    head: Cell<Option<TNodePtr>>,
}

impl HtmlParser {
    fn new(input: &str) -> HtmlParser {
        let document = unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(TNode::Document(Document::new()))))
        };

        HtmlParser {
            tokenizer: Tokenizer::new(input),
            open_elements: RefCell::new(vec![document]),
            mode: Cell::new(InsertionMode::Initial),
            prev_mode: Cell::new(InsertionMode::Initial),
            document: Cell::new(document),
            head: Cell::new(None),
        }
    }

    fn parse(self) -> TNodePtr {
        loop {
            match self.step() {
                StepResult::Continue => continue,
                StepResult::Done => break,
            }
        }

        self.document.get()
    }

    fn step(&self) -> StepResult {
        let token = self.tokenizer.next();

        loop {
            match self.process(&self.mode.get(), &token) {
                ProcessResult::Done => return StepResult::Done,
                ProcessResult::Consumed(mode) => {
                    if let Some(mode) = mode {
                        self.prev_mode.set(mode);
                    }

                    return StepResult::Continue;
                }
                ProcessResult::Ignored => return StepResult::Continue,
                ProcessResult::Reprocess(mode) => {
                    self.mode.set(mode);
                }
            }
        }
    }

    fn process(&self, mode: &InsertionMode, token: &Token) -> ProcessResult {
        match mode {
            //  https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
            InsertionMode::Initial => match token {
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE - Ignore the token.
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => ProcessResult::Ignored,
                // TODO: A comment token
                // TODO: A DOCTYPE token
                // Anything else
                // If the document is not an iframe srcdoc document, then this is a parse error; if the parser cannot change the mode flag is false, set the Document to quirks mode.
                // In any case, switch the insertion mode to "before html", then reprocess the token.
                _ => ProcessResult::Reprocess(InsertionMode::BeforeHtml),
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
            InsertionMode::BeforeHtml => match token {
                // TODO: A DOCTYPE token
                // TODO: A comment token
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE - Ignore the token.
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => ProcessResult::Ignored,
                // A start tag whose tag name is "html"
                Token::StartTag(tag) if tag.name == "html" => {
                    // Create an element for the token in the HTML namespace, with the Document as the intended parent.
                    let element = self.create_element_for_token(tag, Some(self.document.get()));
                    // Append it to the Document object.
                    unsafe { self.document.get().as_ref().append_child(element.clone()) };
                    // Put this element in the stack of open elements.
                    self.open_elements.borrow_mut().push(element);
                    // Switch the insertion mode to "before head".
                    ProcessResult::Consumed(Some(InsertionMode::BeforeHead))
                }
                // An end tag whose tag name is one of: "head", "body", "html", "br"
                Token::EndTag(tag)
                    if tag.name == "head"
                        || tag.name == "body"
                        || tag.name == "html"
                        || tag.name == "br" =>
                {
                    // Act as described in the "anything else" entry below.

                    // Create an html element whose node document is the Document object.
                    let element = self.create_element_for_token(
                        &Tag {
                            name: String::from("html"),
                            self_closing: false,
                            attributes: Vec::new(),
                        },
                        Some(self.document.get()),
                    );
                    // Append it to the Document object.
                    unsafe { self.document.get().as_ref().append_child(element.clone()) };
                    // Put this element in the stack of open elements.
                    self.open_elements.borrow_mut().push(element);
                    // Switch the insertion mode to "before head", then reprocess the token.
                    ProcessResult::Reprocess(InsertionMode::BeforeHead)
                }
                // Any other end tag
                Token::EndTag(tag) => {
                    // Parse error. Ignore the token.
                    self.print_parse_error(&format!("Unexpected end tag: {}", tag.name));
                    ProcessResult::Ignored
                }
                // Anything else
                _ => {
                    // Create an html element whose node document is the Document object.
                    let element = self.create_element_for_token(
                        &Tag {
                            name: String::from("html"),
                            self_closing: false,
                            attributes: Vec::new(),
                        },
                        Some(self.document.get()),
                    );
                    // Append it to the Document object.
                    unsafe { self.document.get().as_ref().append_child(element.clone()) };
                    // Put this element in the stack of open elements.
                    self.open_elements.borrow_mut().push(element);
                    // Switch the insertion mode to "before head", then reprocess the token.
                    ProcessResult::Reprocess(InsertionMode::BeforeHead)
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
            InsertionMode::BeforeHead => match token {
                // A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE - Ignore the token.
                Token::Character('\t')
                | Token::Character('\n')
                | Token::Character('\x0C')
                | Token::Character('\r')
                | Token::Character(' ') => ProcessResult::Ignored,
                // TODO: A comment token
                // TODO: A DOCTYPE token
                // A start tag whose tag name is "html"
                Token::StartTag(tag) if tag.name == "html" => {
                    // Process the token using the rules for the "in body" insertion mode.
                    self.process(&InsertionMode::InBody, token)
                }
                // A start tag whose tag name is "head"
                Token::StartTag(tag) if tag.name == "head" => {
                    // Insert an HTML element for the token.
                    let element = self.insert_element_for_token(tag);
                    // Set the head element pointer to the newly created head element.
                    self.head.set(Some(element));
                    // Switch the insertion mode to "in head".
                    ProcessResult::Consumed(Some(InsertionMode::InHead))
                }
                // An end tag whose tag name is one of: "head", "body", "html", "br"
                Token::EndTag(tag)
                    if tag.name == "head"
                        || tag.name == "body"
                        || tag.name == "html"
                        || tag.name == "br" =>
                {
                    // Act as described in the "anything else" entry below.

                    // Insert an HTML element for a "head" start tag token with no attributes.
                    let element = self.insert_element_for_token(&Tag {
                        name: String::from("head"),
                        self_closing: false,
                        attributes: Vec::new(),
                    });
                    // Set the head element pointer to the newly created head element.
                    self.head.set(Some(element));
                    // Switch the insertion mode to "in head".
                    // Reprocess the current token.
                    ProcessResult::Reprocess(InsertionMode::InHead)
                }
                // Any other end tag
                Token::EndTag(tag) => {
                    // Parse error. Ignore the token.
                    self.print_parse_error(&format!("Unexpected end tag: {}", tag.name));
                    ProcessResult::Ignored
                }
                // Anything else
                _ => {
                    // Insert an HTML element for a "head" start tag token with no attributes.
                    let element = self.insert_element_for_token(&Tag {
                        name: String::from("head"),
                        self_closing: false,
                        attributes: Vec::new(),
                    });
                    // Set the head element pointer to the newly created head element.
                    self.head.set(Some(element));
                    // Switch the insertion mode to "in head".
                    // Reprocess the current token.
                    ProcessResult::Reprocess(InsertionMode::InHead)
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
                    if let Token::Character(ch) = token {
                        self.insert_character(ch);
                    }
                    ProcessResult::Consumed(None)
                }
                // TODO: A comment token
                // TODO: A DOCTYPE token
                // A start tag whose tag name is "html"
                Token::StartTag(tag) if tag.name == "html" => {
                    // Process the token using the rules for the "in body" insertion mode.
                    self.process(&InsertionMode::InBody, token)
                }
                // A start tag whose tag name is one of: "base", "basefont", "bgsound", "link"
                Token::StartTag(tag)
                    if tag.name == "base"
                        || tag.name == "basefont"
                        || tag.name == "bgsound"
                        || tag.name == "link" =>
                {
                    // Insert an HTML element for the token.
                    // Immediately pop the current node off the stack of open elements.
                    // Acknowledge the token's self-closing flag, if it is set.
                    self.insert_element_for_token(tag);
                    ProcessResult::Consumed(None)
                }
            },
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-character
    fn insert_character(&self, ch: &char) {
        if let Some(parent) = self.open_elements.borrow().last().copied() {
            unsafe { parent.as_ref().insert_character(*ch, parent) };
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-an-html-element
    fn insert_element_for_token(&self, tag: &Tag) -> TNodePtr {
        let parent = self.open_elements.borrow().last().copied();

        let node = Box::new(TNode::Element(Element::new(
            parent,
            &tag.name,
            tag.attributes
                .iter()
                .map(|attr| Attr::new(&attr.name, &attr.value))
                .collect(),
        )));

        let node_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(node)) };

        if let Some(parent) = parent {
            unsafe { parent.as_ref().append_child(node_ptr) };
        }

        node_ptr
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#create-an-element-for-the-token
    fn create_element_for_token(&self, tag: &Tag, parent: Option<TNodePtr>) -> TNodePtr {
        let node = Box::new(TNode::Element(Element::new(
            parent,
            &tag.name,
            tag.attributes
                .iter()
                .map(|attr| Attr::new(&attr.name, &attr.value))
                .collect(),
        )));

        unsafe { NonNull::new_unchecked(Box::into_raw(node)) }
    }

    fn print_parse_error(&self, err: &str) {
        println!("[HtmlParser] Parse error: {}", err);
    }
}

pub(crate) fn parse_html(input: &str) -> TNodePtr {
    let parser = HtmlParser::new(input);
    parser.parse()
}
