use crate::dom::html::state::{AttrValueKind, State};
use crate::dom::html::token::{Attribute, Tag, TagKind, Token};
use std::cell::{Cell, RefCell};

enum ProcessResult {
    Continue,
    Switch(State),
    Reconsume(State),
    Emit(Token),
    EmitTokens(Token, Vec<Token>),
    EmitAndReconsume(Token, State),
    EmitAndSwitch(Token, State),
}

/// https://html.spec.whatwg.org/multipage/parsing.html#tokenization
pub(crate) struct Tokenizer {
    input: Vec<char>,
    pos: Cell<usize>,
    reconsume: Cell<bool>,
    state: Cell<State>,
    pending_tokens: RefCell<Vec<Token>>,
    cur_tag_kind: Cell<TagKind>,
    cur_tag_name: RefCell<String>,
    cur_tag_self_closing: Cell<bool>,
    cur_tag_attributes: RefCell<Vec<Attribute>>,
    cur_attr_name: RefCell<String>,
    cur_attr_value: RefCell<String>,
}

impl Tokenizer {
    pub(crate) fn new(input: &str) -> Tokenizer {
        Tokenizer {
            input: input.chars().collect(),
            pos: Cell::new(0),
            reconsume: Cell::new(false),
            state: Cell::new(State::Data),
            pending_tokens: RefCell::new(Vec::new()),
            cur_tag_kind: Cell::new(TagKind::Start),
            cur_tag_name: RefCell::new(String::new()),
            cur_tag_self_closing: Cell::new(false),
            cur_tag_attributes: RefCell::new(Vec::new()),
            cur_attr_name: RefCell::new(String::new()),
            cur_attr_value: RefCell::new(String::new()),
        }
    }

    fn peek(&self) -> Option<char> {
        if self.reconsume.get() {
            self.reconsume.set(false);
            if self.pos.get() <= 0 {
                return None;
            }

            self.pos.set(self.pos.get() - 1);
        }

        let res = self.input.get(self.pos.get()).copied();

        self.pos.set(self.pos.get() + 1);

        res
    }

    pub(crate) fn next(&self) -> Token {
        if let Some(token) = self.pending_tokens.borrow_mut().pop() {
            return token;
        }

        loop {
            let c = self.peek();

            match self.process(c) {
                ProcessResult::Continue => continue,
                ProcessResult::Switch(state) => self.state.set(state),
                ProcessResult::Reconsume(state) => {
                    self.reconsume.set(true);
                    self.state.set(state);
                }
                ProcessResult::Emit(token) => {
                    return token;
                }
                ProcessResult::EmitTokens(token, tokens) => {
                    self.pending_tokens.replace(tokens);
                    return token;
                }
                ProcessResult::EmitAndReconsume(token, state) => {
                    self.reconsume.set(true);
                    self.state.set(state);
                    return token;
                }
                ProcessResult::EmitAndSwitch(token, state) => {
                    self.state.set(state);
                    return token;
                }
            }
        }
    }

    fn process(&self, c: Option<char>) -> ProcessResult {
        match self.state.get() {
            // https://html.spec.whatwg.org/multipage/parsing.html#data-state
            State::Data => match c {
                Some(ch) => match ch {
                    // TODO: U+0026 AMPERSAND (&)
                    // U+003C LESS-THAN SIGN (<) - Switch to the tag open state.
                    '<' => ProcessResult::Switch(State::TagOpen),
                    // TODO: U+0000 NULL
                    // Anything else - Emit the current input character as a character token.
                    _ => ProcessResult::Emit(Token::Character(ch)),
                },
                // EOF - Emit an end-of-file token.
                None => ProcessResult::Emit(Token::EOF),
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
            State::TagOpen => match c {
                Some(ch) => match ch {
                    // TODO: U+0021 EXCLAMATION MARK (!) - Switch to the markup declaration open state.
                    '!' => ProcessResult::Switch(State::SimpleComment),
                    // U+002F SOLIDUS (/) - Switch to the end tag open state.
                    '/' => ProcessResult::Switch(State::EndTagOpen),
                    // ASCII alpha
                    ch if ch.is_ascii_alphabetic() => {
                        // Create a new start tag token, set its tag name to the empty string.
                        self.create_start_tag();
                        // Reconsume in the tag name state.
                        ProcessResult::Reconsume(State::TagName)
                    }
                    // TODO: U+003F QUESTION MARK (?)
                    '?' => {
                        // This is an unexpected-question-mark-instead-of-tag-name parse error.
                        self.print_parse_error("unexpected-question-mark-instead-of-tag-name");
                        // Create a comment token whose data is the empty string.
                        // Reconsume in the bogus comment state.
                        ProcessResult::Switch(State::SimpleComment)
                    }
                    // Anything else
                    _ => {
                        // This is an invalid-first-character-of-tag-name parse error.
                        self.print_parse_error("invalid-first-character-of-tag-name");
                        // Emit a U+003C LESS-THAN SIGN character token.
                        // Reconsume in the data state.
                        ProcessResult::EmitAndReconsume(Token::Character('<'), State::Data)
                    }
                },
                // EOF
                None => {
                    // This is an eof-before-tag-name parse error.
                    self.print_parse_error("eof-before-tag-name");
                    // Emit a U+003C LESS-THAN SIGN character token and an end-of-file token.
                    ProcessResult::EmitTokens(Token::Character('<'), vec![Token::EOF])
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
            State::EndTagOpen => match c {
                Some(ch) => match ch {
                    // ASCII alpha
                    ch if ch.is_ascii_alphabetic() => {
                        // Create a new end tag token, set its tag name to the empty string.
                        self.create_end_tag();
                        // Reconsume in the tag name state.
                        ProcessResult::Reconsume(State::TagName)
                    }
                    // U+003E GREATER-THAN SIGN (>)
                    '>' => {
                        // This is a missing-end-tag-name parse error.
                        self.print_parse_error("missing-end-tag-name");
                        // Switch to the data state.
                        ProcessResult::Switch(State::Data)
                    }
                    // Anything else
                    _ => {
                        // This is an invalid-first-character-of-tag-name parse error.
                        self.print_parse_error("invalid-first-character-of-tag-name");
                        // TODO: Create a comment token whose data is the empty string.
                        // Reconsume in the bogus comment state.
                        ProcessResult::Switch(State::SimpleComment)
                    }
                },
                // EOF
                None => {
                    // This is an eof-before-tag-name parse error.
                    self.print_parse_error("eof-before-tag-name");
                    // Emit a U+003C LESS-THAN SIGN character token, a U+002F SOLIDUS character token and an end-of-file token.
                    ProcessResult::EmitTokens(
                        Token::Character('<'),
                        vec![Token::EOF, Token::Character('/')],
                    )
                }
            },
            State::TagName => match c {
                Some(ch) => match ch {
                    // U+0009 CHARACTER TABULATION (tab) | U+000A LINE FEED (LF) | U+000C FORM FEED (FF) | U+0020 SPACE - Switch to the before attribute name state.
                    '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Switch(State::BeforeAttributeName),
                    // U+002F SOLIDUS (/) - Switch to the self-closing start tag state.
                    '/' => ProcessResult::Switch(State::SelfClosingStartTag),
                    // U+003E GREATER-THAN SIGN (>)
                    '>' => {
                        // Switch to the data state.
                        // Emit the current tag token.
                        ProcessResult::EmitAndSwitch(self.current_tag_token(), State::Data)
                    }
                    // TODO: U+0000 NULL
                    // Anything else
                    _ => {
                        // ASCII upper alpha - Append the lowercase version of the current input character (add 0x0020 to the character's code point) to the current tag token's tag name.
                        // Append the current input character to the current tag token's tag name.
                        self.cur_tag_name.borrow_mut().push(ch.to_ascii_lowercase());
                        ProcessResult::Continue
                    }
                },
                // EOF
                None => {
                    // This is an eof-in-tag parse error.
                    self.print_parse_error("eof-in-tag");
                    // Emit an end-of-file token.
                    ProcessResult::Emit(Token::EOF)
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
            State::BeforeAttributeName => match c {
                Some(ch) => match ch {
                    // U+0009 CHARACTER TABULATION (tab) | U+000A LINE FEED (LF) | U+000C FORM FEED (FF) | U+0020 SPACE - Ignore the character.
                    '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Continue,
                    // U+002F SOLIDUS (/) | U+003E GREATER-THAN SIGN (>) - Reconsume in the after attribute name state.
                    '/' | '>' => ProcessResult::Reconsume(State::AfterAttributeName),
                    // U+003D EQUALS SIGN (=)
                    '=' => {
                        // This is an unexpected-equals-sign-before-attribute-name parse error.
                        self.print_parse_error("unexpected-equals-sign-before-attribute-name");
                        // Start a new attribute in the current tag token.
                        self.create_attr();
                        // Set that attribute's name to the current input character, and its value to the empty string.
                        self.cur_tag_name.borrow_mut().push(ch);
                        // Switch to the attribute name state.
                        ProcessResult::Switch(State::AttributeName)
                    }
                    // Anything else
                    _ => {
                        // Start a new attribute in the current tag token.
                        // Set that attribute name and value to the empty string.
                        self.create_attr();
                        // Reconsume in the attribute name state.
                        ProcessResult::Reconsume(State::AttributeName)
                    }
                },
                // EOF - Reconsume in the after attribute name state.
                None => ProcessResult::Reconsume(State::AfterAttributeName),
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
            State::AttributeName => match c {
                // TODO: When the user agent leaves the attribute name state (and before emitting the tag token, if appropriate), the complete attribute's name must be compared to the other attributes on the same token; if there is already an attribute on the token with the exact same name, then this is a duplicate-attribute parse error and the new attribute must be removed from the token. If an attribute is so removed from a token, it, and the value that gets associated with it, if any, are never subsequently used by the parser, and are therefore effectively discarded. Removing the attribute in this way does not change its status as the "current attribute" for the purposes of the tokenizer, however.
                Some(ch) => match ch {
                    // U+0009 CHARACTER TABULATION (tab) | U+000A LINE FEED (LF) | U+000C FORM FEED (FF) | U+0020 SPACE | U+002F SOLIDUS (/) | U+003E GREATER-THAN SIGN (>) - Reconsume in the after attribute name state.
                    '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                        ProcessResult::Reconsume(State::AfterAttributeName)
                    }
                    // U+003D EQUALS SIGN (=) - Switch to the before attribute value state.
                    '=' => ProcessResult::Switch(State::BeforeAttributeValue),
                    // TODO: U+0000 NULL
                    // U+0022 QUOTATION MARK (") | U+0027 APOSTROPHE (') | U+003C LESS-THAN SIGN (<)
                    '"' | '\'' | '<' => {
                        // This is an unexpected-character-in-attribute-name parse error.
                        self.print_parse_error("unexpected-character-in-attribute-name");
                        // Treat it as per the "anything else" entry below.
                        self.cur_attr_name
                            .borrow_mut()
                            .push(ch.to_ascii_lowercase());
                        ProcessResult::Continue
                    }
                    // Anything else - Append the current input character to the current attribute's name.
                    _ => {
                        // ASCII upper alpha - Append the lowercase version of the current input character (add 0x0020 to the character's code point) to the current attribute's name.
                        self.cur_attr_name
                            .borrow_mut()
                            .push(ch.to_ascii_lowercase());
                        ProcessResult::Continue
                    }
                },
                // EOF - Reconsume in the after attribute name state.
                None => ProcessResult::Reconsume(State::AfterAttributeName),
            },
            State::AfterAttributeName => match c {
                Some(ch) => match ch {
                    // U+0009 CHARACTER TABULATION (tab) | U+000A LINE FEED (LF) | U+000C FORM FEED (FF) | U+0020 SPACE - Ignore the character.
                    '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Continue,
                    // U+002F SOLIDUS (/) - Switch to the self-closing start tag state.
                    '/' => ProcessResult::Switch(State::SelfClosingStartTag),
                    // U+003D EQUALS SIGN (=) - Switch to the before attribute value state.
                    '=' => ProcessResult::Switch(State::BeforeAttributeValue),
                    // U+003E GREATER-THAN SIGN (>) - Switch to the data state. Emit the current tag token.
                    '>' => ProcessResult::EmitAndSwitch(self.current_tag_token(), State::Data),
                    // Anything else
                    _ => {
                        // Start a new attribute in the current tag token.
                        // Set that attribute name and value to the empty string.
                        self.create_attr();
                        // Reconsume in the attribute name state.
                        ProcessResult::Reconsume(State::AttributeName)
                    }
                },
                // EOF
                None => {
                    // This is an eof-in-tag parse error.
                    self.print_parse_error("eof-in-tag");
                    // Emit an end-of-file token.
                    ProcessResult::Emit(Token::EOF)
                }
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
            State::BeforeAttributeValue => match c {
                Some(ch) => match ch {
                    // U+0009 CHARACTER TABULATION (tab) | U+000A LINE FEED (LF) | U+000C FORM FEED (FF) | U+0020 SPACE - Ignore the character.
                    '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Continue,
                    // U+0022 QUOTATION MARK (") - Switch to the attribute value (double-quoted) state.
                    '"' => {
                        ProcessResult::Switch(State::AttributeValue(AttrValueKind::DoubleQuoted))
                    }
                    // U+0027 APOSTROPHE (') - Switch to the attribute value (single-quoted) state.
                    '\'' => {
                        ProcessResult::Switch(State::AttributeValue(AttrValueKind::SingleQuoted))
                    }
                    // U+003E GREATER-THAN SIGN (>)
                    '>' => {
                        // This is a missing-attribute-value parse error.
                        self.print_parse_error("missing-attribute-value");
                        // Switch to the data state.
                        // Emit the current tag token.
                        ProcessResult::EmitAndSwitch(self.current_tag_token(), State::Data)
                    }
                    // Anything else - Reconsume in the attribute value (unquoted) state.
                    _ => ProcessResult::Reconsume(State::AttributeValue(AttrValueKind::Unquoted)),
                },
                // Anything else - Reconsume in the attribute value (unquoted) state.
                None => ProcessResult::Reconsume(State::AttributeValue(AttrValueKind::Unquoted)),
            },
            // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
            State::AttributeValue(AttrValueKind::DoubleQuoted) => match c {
                Some(ch) => match ch {
                    // U+0022 QUOTATION MARK (") - Switch to the after attribute value (quoted) state.
                    '"' => ProcessResult::Switch(State::AfterAttributeValueQuoted),
                    // TODO: U+0026 AMPERSAND (&)
                    // TODO: U+0000 NULL
                    // Anything else - Append the current input character to the current attribute's value.
                    _ => {
                        self.cur_attr_value.borrow_mut().push(ch);
                        ProcessResult::Continue
                    }
                },
                // EOF
                None => {
                    // This is an eof-in-tag parse error.
                    self.print_parse_error("eof-in-tag");
                    // Emit an end-of-file token.
                    ProcessResult::Emit(Token::EOF)
                }
            },
            // TODO: Comment & Bogus Comment
            State::SimpleComment => match c {
                Some(ch) => match ch {
                    '>' => ProcessResult::Switch(State::Data),
                    _ => ProcessResult::Continue,
                },
                None => {
                    self.print_parse_error("eof-in-comment");
                    ProcessResult::Emit(Token::EOF)
                }
            },
        }
    }

    fn create_attr(&self) {
        self.append_cur_attr();
        // self.cur_attr_name.replace(String::new());
        // self.cur_attr_value.replace(String::new());
    }

    fn create_start_tag(&self) {
        self.cur_tag_kind.set(TagKind::Start);
        self.create_tag();
    }

    fn create_end_tag(&self) {
        self.cur_tag_kind.set(TagKind::End);
        self.create_tag();
    }

    fn create_tag(&self) {
        self.cur_tag_name.replace(String::new());
        self.cur_tag_self_closing.set(false);
        self.cur_tag_attributes.replace(Vec::new());
    }

    fn append_cur_attr(&self) {
        self.cur_tag_attributes.borrow_mut().push(Attribute {
            name: self.cur_attr_name.take(),
            value: self.cur_attr_value.take(),
        });
    }

    fn current_tag_token(&self) -> Token {
        self.append_cur_attr();

        self.cur_tag_attributes
            .borrow_mut()
            .retain(|attr| !attr.name.is_empty());

        let tag = Tag::new(
            self.cur_tag_name.take(),
            self.cur_tag_self_closing.get(),
            self.cur_tag_attributes.take(),
        );

        match self.cur_tag_kind.get() {
            TagKind::Start => Token::StartTag(tag),
            TagKind::End => Token::EndTag(tag),
        }
    }

    fn print_parse_error(&self, err: &str) {
        println!("[Tokenizer] Parse error: {}", err);
    }
}
