use std::cell::{Cell, RefCell};
use std::mem;
use std::ops::DerefMut;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
enum TagKind {
    #[default]
    StartTag,
    EndTag,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Attribute {
    name: String,
    value: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Tag {
    kind: TagKind,
    name: String,
    self_closing: bool,
    attributes: Vec<Attribute>,
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Tag(Tag),
    Character(char),
    Comment(String),
    EOF,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Tag(tag) => write!(
                f,
                "tag name={} kind={:#?} self_closing={} attributes={:?}",
                tag.name, tag.kind, tag.self_closing, tag.attributes
            ),
            Token::Character(ch) => write!(f, "char={}", ch),
            Token::Comment(comment) => write!(f, "comment={}", comment),
            Token::EOF => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    AfterAttributeValueQuoted,
    EndTagOpen,
    MarkupDeclarationOpen,
    CommentStart,
    Comment,
    CommentEnd,
    BogusComment,
}

struct Tokenizer {
    input: Vec<char>,
    pos: Cell<usize>,
    reconsume: Cell<bool>,
    state: Cell<State>,
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
            pending_tokens: RefCell::new(Vec::new()),
            cur_tag: RefCell::new(Tag::default()),
            cur_comment: RefCell::new(String::new()),
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
                        // TODO: 1. U+0026 AMPERSAND (&) - Set the return state to the data state. Switch to the character reference state. 2. U+0000 NULL - This is an unexpected-null-character parse error. Emit the current input character as a character token.
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
                            self.state.set(State::EndTagOpen);
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
                // https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
                State::EndTagOpen => match c {
                    Some(ch) => match ch {
                        '>' => {
                            // This is a missing-end-tag-name parse error. Switch to the data state.
                            self.state.set(State::Data);
                        }
                        _ => {
                            if ch.is_ascii_alphabetic() {
                                // Create a new end tag token, set its tag name to the empty string. Reconsume in the tag name state.
                                self.create_tag(TagKind::EndTag);
                                self.reconsume.set(true);
                                self.state.set(State::TagName);
                                continue;
                            }

                            // This is an invalid-first-character-of-tag-name parse error. Create a comment token whose data is the empty string. Reconsume in the bogus comment state.
                            self.create_comment();
                            self.reconsume.set(true);
                            self.state.set(State::BogusComment);
                        }
                    },
                    None => {
                        // This is an eof-before-tag-name parse error.
                        // Emit a U+003C LESS-THAN SIGN character token, a U+002F SOLIDUS character token and an end-of-file token.
                        self.pending_tokens.borrow_mut().push(Token::EOF);
                        self.pending_tokens.borrow_mut().push(Token::Character('/'));
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
                        // TODO: U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the current tag token's tag name.
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
                    // TODO: When the user agent leaves the attribute name state (and before emitting the tag token, if appropriate), the complete attribute's name must be compared to the other attributes on the same token; if there is already an attribute on the token with the exact same name, then this is a duplicate-attribute parse error and the new attribute must be removed from the token.
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
                            self.cur_tag
                                .borrow_mut()
                                .attributes
                                .last_mut()
                                .expect("[State::AttributeName] last attribute is invalid")
                                .name
                                .push(ch);
                        }
                        // TODO: U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the current attribute's name.
                        _ => {
                            // ASCII upper alpha - Append the lowercase version of the current input character (add 0x0020 to the character's code point) to the current attribute's name.
                            // Anything else - Append the current input character to the current attribute's name.
                            self.cur_tag
                                .borrow_mut()
                                .attributes
                                .last_mut()
                                .expect("[State::AttributeName] last attribute is invalid")
                                .name
                                .push(ch.to_ascii_lowercase());
                        }
                    },
                    None => {
                        // Reconsume in the after attribute name state.
                        self.reconsume.set(true);
                        self.state.set(State::AfterAttributeName);
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
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
                // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
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
                // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
                State::AttributeValue(AttrValueKind::DoubleQuoted) => match c {
                    Some(ch) => match ch {
                        '"' => {
                            // Switch to the after attribute value (quoted) state.
                            self.state.set(State::AfterAttributeValueQuoted);
                        }
                        // TODO: 1. U+0026 AMPERSAND (&) - Set the return state to the attribute value (double-quoted) state. Switch to the character reference state. 2. U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the current attribute's value.
                        _ => {
                            // Append the current input character to the current attribute's value.
                            self.cur_tag.borrow_mut().attributes.last_mut().expect("[State::AttributeValue(AttrValueKind::DoubleQuoted)] last attribute is invalid").value.push(ch);
                        }
                    },
                    None => {
                        // This is an eof-in-tag parse error. Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(single-quoted)-state
                State::AttributeValue(AttrValueKind::SingleQuoted) => match c {
                    Some(ch) => match ch {
                        '\'' => {
                            // Switch to the after attribute value (quoted) state.
                            self.state.set(State::AfterAttributeValueQuoted);
                        }
                        // TODO: 1. U+0026 AMPERSAND (&) - Set the return state to the attribute value (double-quoted) state. Switch to the character reference state. 2. U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the current attribute's value.
                        _ => {
                            // Append the current input character to the current attribute's value.
                            self.cur_tag.borrow_mut().attributes.last_mut().expect("[State::AttributeValue(AttrValueKind::DoubleQuoted)] last attribute is invalid").value.push(ch);
                        }
                    },
                    None => {
                        // This is an eof-in-tag parse error. Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(unquoted)-state
                State::AttributeValue(AttrValueKind::Unquoted) => match c {
                    Some(ch) => match ch {
                        '\t' | '\n' | '\x0C' | ' ' => {
                            // Switch to the before attribute name state.
                            self.state.set(State::BeforeAttributeName);
                        }
                        // TODO: U+0026 AMPERSAND (&) - Set the return state to the attribute value (unquoted) state. Switch to the character reference state.
                        '>' => {
                            // Switch to the data state. Emit the current tag token.
                            self.state.set(State::Data);
                            return self.emit_tag();
                        }
                        // TODO: U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the current attribute's value.
                        '"' | '\'' | '<' | '=' | '`' => {
                            // This is an unexpected-character-in-unquoted-attribute-value parse error.
                            // Treat it as per the "anything else" entry below.
                            self.cur_tag.borrow_mut().attributes.last_mut().expect("[State::AttributeValue(AttrValueKind::Unquoted)] last attribute is invalid").value.push(ch);
                        }
                        _ => {
                            // Append the current input character to the current attribute's value.
                            self.cur_tag.borrow_mut().attributes.last_mut().expect("[State::AttributeValue(AttrValueKind::Unquoted)] last attribute is invalid").value.push(ch);
                        }
                    },
                    None => {
                        // This is an eof-in-tag parse error. Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-value-(quoted)-state
                State::AfterAttributeValueQuoted => match c {
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
                        _ => {
                            // This is a missing-whitespace-between-attributes parse error.
                            // Reconsume in the before attribute name state.
                            self.reconsume.set(true);
                            self.state.set(State::BeforeAttributeName);
                        }
                    },
                    None => {
                        // This is an eof-in-tag parse error. Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
                State::SelfClosingStartTag => match c {
                    Some(ch) => match ch {
                        '>' => {
                            // Set the self-closing flag of the current tag token.
                            // Switch to the data state.
                            // Emit the current tag token.
                            self.cur_tag.borrow_mut().self_closing = true;
                            self.state.set(State::Data);
                            return self.emit_tag();
                        }
                        _ => {
                            // This is an unexpected-solidus-in-tag parse error.
                            // Reconsume in the before attribute name state.
                            self.reconsume.set(true);
                            self.state.set(State::BeforeAttributeName);
                        }
                    },
                    None => {
                        // This is an eof-in-tag parse error. Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#bogus-comment-state
                State::BogusComment => match c {
                    Some(ch) => match ch {
                        '>' => {
                            // Switch to the data state. Emit the current comment token.
                            self.state.set(State::Data);
                            return self.emit_comment();
                        }
                        // TODO: U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the comment token's data.
                        _ => {
                            // Append the current input character to the comment token's data.
                            self.cur_comment.borrow_mut().push(ch);
                        }
                    },
                    None => {
                        // Emit the comment. Emit an end-of-file token.
                        self.pending_tokens.borrow_mut().push(Token::EOF);
                        return self.emit_comment();
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#markup-declaration-open-state
                State::MarkupDeclarationOpen => match c {
                    Some(_) => {
                        // TODO: 1. Two U+002D HYPHEN-MINUS characters (-) - Consume those two characters, create a comment token whose data is the empty string, and switch to the comment start state. 2. ASCII case-insensitive match for the word "DOCTYPE" - Consume those characters and switch to the DOCTYPE state. 3. The string "[CDATA[" (the five uppercase letters "CDATA" with a U+005B LEFT SQUARE BRACKET character before and after) - Consume those characters. If there is an adjusted current node and it is not an element in the HTML namespace, then switch to the CDATA section state. Otherwise, this is a cdata-in-html-content parse error. Create a comment token whose data is the "[CDATA[" string. Switch to the bogus comment state.
                        self.create_comment();
                        self.reconsume.set(true);
                        self.state.set(State::CommentStart);
                    }
                    None => {
                        // This is an incorrectly-opened-comment parse error.
                        // Create a comment token whose data is the empty string.
                        // Switch to the bogus comment state (don't consume anything in the current state).
                        self.create_comment();
                        self.state.set(State::BogusComment);
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#comment-start-state
                State::CommentStart => match c {
                    Some(ch) => match ch {
                        // TODO: U+002D HYPHEN-MINUS (-) - Switch to the comment start dash state.
                        '>' => {
                            // This is an abrupt-closing-of-empty-comment parse error.
                            // Switch to the data state. Emit the current comment token.
                            self.state.set(State::Data);
                            return self.emit_comment();
                        }
                        _ => {
                            // Reconsume in the comment state.
                            self.reconsume.set(true);
                            self.state.set(State::Comment);
                        }
                    },
                    None => {
                        // Reconsume in the comment state.
                        self.reconsume.set(true);
                        self.state.set(State::Comment);
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#comment-state
                State::Comment => match c {
                    Some(ch) => match ch {
                        // TODO: 1. U+003C LESS-THAN SIGN (<) - Append the current input character to the comment token's data. Switch to the comment less-than sign state. 2. U+002D HYPHEN-MINUS (-) - Switch to the comment end dash state. 3. U+0000 NULL - This is an unexpected-null-character parse error. Append a U+FFFD REPLACEMENT CHARACTER character to the comment token's data.
                        '>' => {
                            self.reconsume.set(true);
                            self.state.set(State::CommentEnd);
                        }
                        _ => {
                            // Append the current input character to the comment token's data.
                            self.cur_comment.borrow_mut().push(ch);
                        }
                    },
                    None => {
                        // This is an eof-in-comment parse error.
                        // Emit the current comment token. Emit an end-of-file token.
                        self.pending_tokens.borrow_mut().push(Token::EOF);
                        return self.emit_comment();
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#comment-end-state
                State::CommentEnd => match c {
                    Some(ch) => match ch {
                        '>' => {
                            // Switch to the data state. Emit the current comment token.
                            self.state.set(State::Data);
                            return self.emit_comment();
                        }
                        // TODO: 1. U+0021 EXCLAMATION MARK (!) - Switch to the comment end bang state. 2. U+002D HYPHEN-MINUS (-) - Append a U+002D HYPHEN-MINUS character (-) to the comment token's data.
                        _ => {
                            // Append two U+002D HYPHEN-MINUS characters (-) to the comment token's data. Reconsume in the comment state.
                            self.cur_comment.borrow_mut().push_str("--");
                            self.reconsume.set(true);
                            self.state.set(State::Comment);
                        }
                    },
                    None => {
                        // This is an eof-in-comment parse error. Emit the current comment token. Emit an end-of-file token.
                        self.pending_tokens.borrow_mut().push(Token::EOF);
                        return self.emit_comment();
                    }
                },
            }
        }
    }

    fn create_comment(&self) {
        *self.cur_comment.borrow_mut() = String::new();
    }

    fn emit_comment(&self) -> Token {
        return Token::Comment(mem::take(self.cur_comment.borrow_mut().deref_mut()));
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

/// Generated by Gemini 3 Pro
#[cfg(test)]
mod tests {
    use super::*;

    fn collect_tokens(input: &str) -> Vec<Token> {
        let tokenizer = Tokenizer::new(input);
        let mut tokens = Vec::new();
        loop {
            let token = tokenizer.next();
            let is_eof = matches!(token, Token::EOF);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn attr(name: &str, value: &str) -> Attribute {
        Attribute {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    fn start_tag(name: &str, attributes: Vec<Attribute>, self_closing: bool) -> Token {
        Token::Tag(Tag {
            kind: TagKind::StartTag,
            name: name.to_string(),
            self_closing,
            attributes,
        })
    }

    fn end_tag(name: &str) -> Token {
        Token::Tag(Tag {
            kind: TagKind::EndTag,
            name: name.to_string(),
            self_closing: false,
            attributes: Vec::new(),
        })
    }

    #[test]
    fn test_basic_text() {
        let tokens = collect_tokens("abc");
        assert_eq!(
            tokens,
            vec![
                Token::Character('a'),
                Token::Character('b'),
                Token::Character('c'),
                Token::EOF
            ]
        );
    }

    #[test]
    fn test_basic_tags() {
        let tokens = collect_tokens("<div></div>");
        assert_eq!(
            tokens,
            vec![start_tag("div", vec![], false), end_tag("div"), Token::EOF]
        );
    }

    #[test]
    fn test_tag_case_insensitivity() {
        // 标签名应自动转小写
        let tokens = collect_tokens("<DIV></div >");
        assert_eq!(
            tokens,
            vec![start_tag("div", vec![], false), end_tag("div"), Token::EOF]
        );
    }

    #[test]
    fn test_attributes_mixed() {
        // 测试双引号、无引号和不同属性情况
        let tokens = collect_tokens("<div id=\"test\" class=foo checked>");

        let expected_attrs = vec![
            attr("id", "test"),
            attr("class", "foo"),
            attr("checked", ""), // 布尔属性值为空字符串
        ];

        assert_eq!(tokens[0], start_tag("div", expected_attrs, false));
    }

    #[test]
    fn test_attributes_single_quoted() {
        let tokens = collect_tokens("<div id='test'>");
        assert_eq!(tokens[0], start_tag("div", vec![attr("id", "test")], false));
    }

    #[test]
    fn test_self_closing_tag() {
        let tokens = collect_tokens("<br/>");
        assert_eq!(tokens[0], start_tag("br", vec![], true));
    }

    #[test]
    fn test_bogus_comment_xml() {
        // <?xml ...> 应该进入 BogusComment
        let tokens = collect_tokens("<?xml version='1.0'?>");
        assert_eq!(tokens[0], Token::Comment("?xml version='1.0'?".to_string()));
    }

    #[test]
    fn test_bogus_comment_doctype() {
        // 你的实现将 <!DOCTYPE> 简化处理为 BogusComment
        let tokens = collect_tokens("<!DOCTYPE>");
        assert_eq!(tokens[0], Token::Comment("DOCTYPE".to_string()));
    }

    #[test]
    fn test_empty_comment() {
        let tokens = collect_tokens("<!>");
        assert_eq!(tokens[0], Token::Comment("".to_string()));
    }

    #[test]
    fn test_eof_in_tag_edge_case() {
        // 测试 State::EndTagOpen 中的 EOF 处理逻辑
        // 输入 "</"，期望输出 Token('<'), Token('/'), Token(EOF)
        let tokens = collect_tokens("</");
        assert_eq!(
            tokens,
            vec![Token::Character('<'), Token::Character('/'), Token::EOF]
        );
    }

    #[test]
    fn test_invalid_tag_name_start() {
        // 测试 State::TagOpen 中非法字符的回退逻辑
        // 输入 "<4"，期望输出 Token('<'), Token('4'), Token(EOF)
        let tokens = collect_tokens("<4");
        assert_eq!(
            tokens,
            vec![Token::Character('<'), Token::Character('4'), Token::EOF]
        );
    }

    #[test]
    fn test_attribute_value_with_illegal_chars() {
        // 测试 Unquoted Attribute Value 对非法字符的宽容处理
        // <div data=foo"bar> -> value: foo"bar
        let tokens = collect_tokens("<div data=foo\"bar>");

        assert_eq!(
            tokens[0],
            start_tag("div", vec![attr("data", "foo\"bar")], false)
        );
    }
}
