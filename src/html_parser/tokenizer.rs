use std::cell::{Cell, RefCell};
use std::mem;
use std::ops::DerefMut;

#[derive(Debug, Default, Copy, Clone)]
enum TagKind {
    #[default]
    StartTag,
    EndTag,
}

#[derive(Debug, Default)]
struct Attribute {
    name: String,
    value: String,
}

#[derive(Debug, Default)]
struct Tag {
    kind: TagKind,
    name: String,
    self_closing: bool,
    attributes: Vec<Attribute>,
}

enum Token {
    Tag(Tag),
    Character(char),
    Comment(String),
    EOF,
}

#[derive(Debug, Copy, Clone)]
enum AttrValueKind {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

#[derive(Debug, Copy, Clone)]
enum State {
    Data,
    TagOpen,
    TagName,
    SelfClosingStartTag,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValue(AttrValueKind),
    EngTagOpen,
    BogusComment,
    MarkupDeclarationOpen,
}

struct Tokenizer {
    input: Vec<char>,
    pos: Cell<usize>,
    reconsume: Cell<bool>,
    state: Cell<State>,
    cur_chars: RefCell<Vec<char>>,
    pending_tokens: RefCell<Vec<Token>>,
    cur_tag: RefCell<Tag>,
    cur_comment: RefCell<String>,
}

impl Tokenizer {
    fn new(input: &str) -> Tokenizer {
        Tokenizer {
            input: input.chars().collect(),
            pos: Cell::new(0),
            reconsume: Cell::new(false),
            state: Cell::new(State::Data),
            cur_chars: RefCell::new(Vec::new()),
            pending_tokens: RefCell::new(Vec::new()),
            cur_tag: RefCell::new(Tag::default()),
            cur_comment: RefCell::new(String::new()),
        }
    }

    fn peek(&self) -> Option<char> {
        if self.reconsume.get() {
            self.reconsume.set(false);
        } else {
            self.pos.set(self.pos.get() + 1);
        }

        self.input.get(self.pos.get()).copied()
    }

    fn next(&self) -> Token {
        if let Some(token) = self.pending_tokens.borrow_mut().pop() {
            return token;
        }

        loop {
            let c = self.peek();

            match self.state.get() {
                // https://html.spec.whatwg.org/multipage/parsing.html#data-state
                State::Data => match c {
                    Some(ch) => match ch {
                        '<' => {
                            // Switch to the tag open state.
                            self.state.set(State::TagOpen);
                        }
                        // TODO:
                        // U+0026 AMPERSAND (&) - Set the return state to the data state. Switch to the character reference state.
                        // U+0000 NULL - This is an unexpected-null-character parse error. Emit the current input character as a character token.
                        _ => {
                            // Emit the current input character as a character token.
                            return Token::Character(ch);
                        }
                    },
                    None => {
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
                State::TagOpen => match c {
                    Some(ch) => match ch {
                        '!' => {
                            // Switch to the markup declaration open state.
                            self.state.set(State::MarkupDeclarationOpen);
                        }
                        '/' => {
                            // Switch to the end tag open state.
                            self.state.set(State::EngTagOpen);
                        }
                        '?' => {
                            // This is an unexpected-question-mark-instead-of-tag-name parse error.
                            // Create a comment token whose data is the empty string.
                            // Reconsume in the bogus comment state.
                            *self.cur_comment.borrow_mut() = String::new();
                            self.reconsume.set(true);
                            self.state.set(State::BogusComment);
                        }
                        _ => {
                            if ch.is_ascii_alphabetic() {
                                // Create a new start tag token, set its tag name to the empty string.
                                // Reconsume in the tag name state.
                                self.create_tag(TagKind::StartTag);
                                self.reconsume.set(true);
                                self.state.set(State::TagName);
                                continue;
                            }

                            // This is an invalid-first-character-of-tag-name parse error.
                            // Emit a U+003C LESS-THAN SIGN character token.
                            // Reconsume in the data state.
                            self.reconsume.set(true);
                            self.state.set(State::Data);
                            return Token::Character('<');
                        }
                    },
                    None => {
                        // This is an eof-before-tag-name parse error.
                        // Emit a U+003C LESS-THAN SIGN character token and an end-of-file token.
                        self.pending_tokens.borrow_mut().push(Token::EOF);
                        return Token::Character('<');
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
                State::TagName => match c {
                    Some(ch) => match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {
                            // Switch to the before attribute name state.
                            self.state.set(State::BeforeAttributeName);
                        }
                        '/' => {
                            // Switch to the self-closing start tag state.
                            self.state.set(State::SelfClosingStartTag);
                        }
                        '>' => {
                            // Switch to the data state. Emit the current tag token.
                            self.state.set(State::Data);
                            return self.emit_tag();
                        }
                        // TODO:
                        // U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the current tag token's tag name.
                        _ => {
                            // ASCII upper alpha - Append the lowercase version of the current input character (add 0x0020 to the character's code point) to the current tag token's tag name.
                            // Anything else - Append the current input character to the current tag token's tag name.
                            self.cur_tag.borrow_mut().name.push(ch.to_ascii_lowercase());
                        }
                    },
                    None => {
                        // This is an eof-in-tag parse error.
                        // Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
                State::BeforeAttributeName => match c {
                    Some(ch) => match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {
                            // Ignore the character.
                        }
                        '/' | '>' => {
                            // Reconsume in the after attribute name state.
                            self.reconsume.set(true);
                            self.state.set(State::AfterAttributeName);
                        }
                        '=' => {
                            // This is an unexpected-equals-sign-before-attribute-name parse error.
                            // Start a new attribute in the current tag token.
                            // Set that attribute's name to the current input character, and its value to the empty string.
                            // Switch to the attribute name state.
                            self.create_attr(Some('='));
                            self.state.set(State::AttributeName);
                        }
                        _ => {
                            // Start a new attribute in the current tag token.
                            // Set that attribute name and value to the empty string.
                            // Reconsume in the attribute name state.
                            self.create_attr(None);
                            self.reconsume.set(true);
                            self.state.set(State::AttributeName);
                        }
                    },
                    None => {
                        // Reconsume in the after attribute name state.
                        self.reconsume.set(true);
                        self.state.set(State::AfterAttributeName);
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
                State::AttributeName => match c {
                    Some(ch) => match ch {
                        '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                            // Reconsume in the after attribute name state.
                            self.reconsume.set(true);
                            self.state.set(State::AfterAttributeName);
                        }
                        '=' => {
                            // Switch to the before attribute value state.
                            self.state.set(State::BeforeAttributeValue);
                        }
                        '"' | '\'' | '<' => {
                            // This is an unexpected-character-in-attribute-name parse error. Treat it as per the "anything else" entry below.
                            self.cur_tag.borrow_mut().attributes.last_mut().expect("[State::AttributeName] self.cur_tag.borrow_mut().attributes.last() is invalid").name.push(ch);
                        }
                        // TODO:
                        // U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the current attribute's name.
                        _ => {
                            // ASCII upper alpha - Append the lowercase version of the current input character (add 0x0020 to the character's code point) to the current attribute's name.
                            // Anything else - Append the current input character to the current attribute's name.
                            self.cur_tag.borrow_mut().attributes.last_mut().expect("[State::AttributeName] self.cur_tag.borrow_mut().attributes.last() is invalid").name.push(ch.to_ascii_lowercase());
                        }
                    },
                    None => {
                        // Reconsume in the after attribute name state.
                        self.reconsume.set(true);
                        self.state.set(State::AfterAttributeName);
                    }
                },
                State::AfterAttributeName => match c {
                    Some(ch) => match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {
                            // Ignore the character.
                        }
                        '/' => {
                            // Switch to the self-closing start tag state.
                            self.state.set(State::SelfClosingStartTag);
                        }
                        '=' => {
                            // Switch to the before attribute value state.
                            self.state.set(State::BeforeAttributeValue);
                        }
                        '>' => {
                            // Switch to the data state. Emit the current tag token.
                            self.state.set(State::Data);
                            return self.emit_tag();
                        }
                        _ => {
                            // Start a new attribute in the current tag token.
                            // Set that attribute name and value to the empty string.
                            // Reconsume in the attribute name state.
                            self.create_attr(None);
                            self.reconsume.set(true);
                            self.state.set(State::AttributeName);
                        }
                    },
                    None => {
                        // This is an eof-in-tag parse error. Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                State::BeforeAttributeValue => match c {
                    Some(ch) => match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {
                            // Ignore the character.
                        }
                        '"' => {
                            // Switch to the attribute value (double-quoted) state.
                            self.state
                                .set(State::AttributeValue(AttrValueKind::DoubleQuoted));
                        }
                        '\'' => {
                            // Switch to the attribute value (single-quoted) state.
                            self.state
                                .set(State::AttributeValue(AttrValueKind::SingleQuoted));
                        }
                        '>' => {
                            // This is a missing-attribute-value parse error.
                            // Switch to the data state.
                            // Emit the current tag token.
                            self.state.set(State::Data);
                            return self.emit_tag();
                        }
                        _ => {
                            // Reconsume in the attribute value (unquoted) state.
                            self.reconsume.set(true);
                            self.state
                                .set(State::AttributeValue(AttrValueKind::Unquoted));
                        }
                    },
                    None => {
                        // Reconsume in the attribute value (unquoted) state.
                        self.reconsume.set(true);
                        self.state
                            .set(State::AttributeValue(AttrValueKind::Unquoted));
                    }
                },
            }
        }
    }

    fn create_attr(&self, c: Option<char>) {
        self.cur_tag.borrow_mut().attributes.push(Attribute {
            name: match c {
                Some(ch) => String::from(ch),
                None => String::new(),
            },
            value: String::new(),
        });
    }

    fn create_tag(&self, kind: TagKind) {
        *self.cur_tag.borrow_mut() = Tag {
            kind,
            name: String::new(),
            self_closing: false,
            attributes: Vec::new(),
        };
    }

    fn emit_tag(&self) -> Token {
        self.cur_tag
            .borrow_mut()
            .attributes
            .retain(|attr| !attr.name.is_empty());

        Token::Tag(mem::take(self.cur_tag.borrow_mut().deref_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        todo!();
    }
}
