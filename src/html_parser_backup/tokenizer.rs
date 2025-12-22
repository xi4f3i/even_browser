use crate::html_parser_backup::{
    state::State,
    token::{Attribute, Tag, Token},
};

#[derive(Debug)]
pub(crate) struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    state: State,
    reconsume_char: Option<char>,
    current_token: Option<Token>,
    pending_tokens: Vec<Token>,
}

impl Tokenizer {
    pub(crate) fn new(input: &str) -> Tokenizer {
        Tokenizer {
            input: input.chars().collect(),
            pos: 0,
            state: State::Data,
            reconsume_char: None,
            current_token: None,
            pending_tokens: Vec::new(),
        }
    }

    fn peek(&mut self) -> Option<char> {
        match self.reconsume_char {
            Some(c) => Some(c),
            None => {
                if self.pos < self.input.len() {
                    let c = self.input[self.pos];
                    self.pos += 1;
                    Some(c)
                } else {
                    None
                }
            }
        }
    }

    fn next_token(&mut self) -> Token {
        if let Some(token) = self.pending_tokens.pop() {
            return token;
        }

        loop {
            let c = self.peek();

            match self.state {
                // https://html.spec.whatwg.org/multipage/parsing.html#data-state
                State::Data => match c {
                    Some('<') => self.state = State::TagOpen,
                    Some(c) => return Token::Character(c),
                    None => return Token::EOF,
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
                State::TagOpen => match c {
                    Some('!') => self.state = State::MarkupDeclarationOpen,
                    Some('/') => self.state = State::EndTagOpen,
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        // Create a new start tag token, set its tag name to the empty string.
                        // Reconsume in the tag name state.
                        self.current_token = Some(Token::StartTag(Tag {
                            tag_name: String::new(),
                            attributes: Vec::new(),
                            self_closing: false,
                        }));
                        self.state = State::TagName;
                        self.reconsume_char = Some(ch);
                    }
                    _ => {
                        // This is an invalid-first-character-of-tag-name parse error.
                        // Emit a U+003C LESS-THAN SIGN character token.
                        // Reconsume in the data state.
                        self.state = State::Data;
                        self.reconsume_char = c;
                        return Token::Character('<');
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
                State::EndTagOpen => match c {
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        // Create a new end tag token, set its tag name to the empty string.
                        // Reconsume in the tag name state.
                        self.current_token = Some(Token::EndTag(String::new()));
                        self.state = State::TagName;
                        self.reconsume_char = Some(ch);
                    }
                    Some('>') => {
                        // This is a missing-end-tag-name parse error.
                        // Switch to the data state.
                        self.state = State::Data;
                    }
                    None => {
                        // This is an eof-before-tag-name parse error.
                        // Emit a U+003C LESS-THAN SIGN character token,
                        // a U+002F SOLIDUS character token and an end-of-file token.
                        self.pending_tokens.push(Token::EOF);
                        self.pending_tokens.push(Token::Character('/'));
                        return Token::Character('<');
                    }
                    Some(ch) => {
                        // This is an invalid-first-character-of-tag-name parse error.
                        // Create a comment token whose data is the empty string.
                        // Reconsume in the bogus comment state.
                        self.current_token = Some(Token::Comment(String::new()));
                        self.state = State::BogusComment;
                        self.reconsume_char = Some(ch);
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
                State::TagName => match c {
                    Some(ch) if ch.is_whitespace() => self.state = State::BeforeAttributeName,
                    Some('/') => self.state = State::SelfClosingStartTag,
                    Some('>') => {
                        // Switch to the data state.
                        // Emit the current tag token.
                        self.state = State::Data;
                        return self.emit_tag_token();
                    }
                    Some(ch) => {
                        // Append the current input character to the current tag token's tag name.
                        // Append the lowercase version of the current input character (add 0x0020 to the character's code point)
                        // to the current tag token's tag name.
                        if let Some(t) = self.current_token.as_mut() {
                            match t {
                                Token::StartTag(t) => t.tag_name.push(ch.to_ascii_lowercase()),
                                Token::EndTag(tag_name) => tag_name.push(ch.to_ascii_lowercase()),
                                _ => {}
                            }
                        }
                    }
                    None => {
                        // This is an eof-in-tag parse error.
                        // Emit an end-of-file token.
                        return Token::EOF;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
                State::BeforeAttributeName => match c {
                    Some(ch) if ch.is_whitespace() => {
                        // Ignore the character.
                    }
                    Some('/') | Some('>') | None => {
                        // Reconsume in the after attribute name state.
                        self.state = State::AfterAttributeName;
                        self.reconsume_char = c;
                    }
                    Some('=') => {
                        // This is an unexpected-equals-sign-before-attribute-name parse error.
                        // Start a new attribute in the current tag token.
                        // Set that attribute's name to the current input character, and its value to the empty string.
                        // Switch to the attribute name state.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                        {
                            tag.attributes.push(Attribute {
                                name: String::from("="),
                                value: String::new(),
                            });
                        }
                        self.state = State::AttributeName;
                    }
                    Some(ch) => {
                        // Start a new attribute in the current tag token.
                        // Set that attribute name and value to the empty string.
                        // Reconsume in the attribute name state.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                        {
                            tag.attributes.push(Attribute {
                                name: String::new(),
                                value: String::new(),
                            });
                        }
                        self.state = State::AttributeName;
                        self.reconsume_char = Some(ch);
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
                State::AttributeName => match c {
                    Some('\t') | Some('\n') | Some('\x0C') | Some(' ') | Some('/') | Some('>')
                    | None => {
                        // Reconsume in the after attribute name state.
                        self.state = State::AfterAttributeName;
                        self.reconsume_char = c;
                    }
                    Some('=') => {
                        // Switch to the before attribute value state.
                        self.state = State::BeforeAttributeValue;
                    }
                    Some('\'') | Some('"') | Some('<') => {
                        // This is an unexpected-character-in-attribute-name parse error.
                        // Treat it as per the "anything else" entry below.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                            && let Some(attr) = tag.attributes.last().as_mut()
                        {
                            attr.name
                                .push(c.expect("State::AttributeName  c is invalid"));
                        }
                    }
                    Some('\0') => {
                        // This is an unexpected-null-character parse error.
                        // Append a U+FFFD REPLACEMENT CHARACTER character to the current attribute's name.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                            && let Some(attr) = tag.attributes.last().as_mut()
                        {
                            attr.name.push('\u{FFFD}');
                        }
                    }
                    Some(ch) => {
                        // Append the current input character to the current attribute's name.
                        // Append the lowercase version of the current input character (add 0x0020 to the character's code point) to the current attribute's name.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                            && let Some(attr) = tag.attributes.last().as_mut()
                        {
                            attr.name.push(ch.to_ascii_lowercase());
                        }
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
                State::AfterAttributeName => match c {
                    Some(ch) if ch.is_whitespace() => {
                        // Ignore the character.
                    }
                    Some('/') => {
                        // Switch to the self-closing start tag state.
                        self.state = State::SelfClosingStartTag;
                    }
                    Some('=') => {
                        // Switch to the before attribute value state.
                        self.state = State::BeforeAttributeValue;
                    }
                    Some('>') => {
                        // Switch to the data state.
                        // Emit the current tag token.
                        self.state = State::Data;
                        self.emit_tag_token();
                    }
                    None => {
                        // This is an eof-in-tag parse error.
                        // Emit an end-of-file token.
                        return Token::EOF;
                    }
                    Some(ch) => {
                        // Start a new attribute in the current tag token.
                        // Set that attribute name and value to the empty string.
                        // Reconsume in the attribute name state.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                        {
                            tag.attributes.push(Attribute {
                                name: String::new(),
                                value: String::new(),
                            });
                        }
                        self.state = State::AttributeName;
                        self.reconsume_char = Some(ch);
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
                State::BeforeAttributeValue => match c {
                    Some(ch) if ch.is_whitespace() => {
                        // Ignore the character.
                    }
                    Some('"') => {
                        // Switch to the attribute value (double-quoted) state.
                        self.state = State::AttributeValueDoubleQuoted;
                    }
                    Some('\'') => {
                        // Switch to the attribute value (double-quoted) state.
                        self.state = State::AttributeValueSingleQuoted;
                    }
                    Some('>') => {
                        // This is a missing-attribute-value parse error.
                        // Switch to the data state.
                        // Emit the current tag token.
                    }
                    Some(ch) => {
                        // Reconsume in the attribute value (unquoted) state.
                        self.state = State::AttributeValueUnquoted;
                        self.reconsume_char = Some(ch);
                    }
                    None => {
                        // Reconsume in the attribute value (unquoted) state.
                        self.state = State::AttributeValueUnquoted;
                        self.reconsume_char = None;
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
                State::AttributeValueDoubleQuoted => match c {
                    Some('"') => {
                        // Switch to the after attribute value (quoted) state.
                        self.state = State::AfterAttributeValueQuoted;
                    }
                    // Some('&') => {
                    // Set the return state to the attribute value (double-quoted) state.
                    // Switch to the character reference state.
                    // }
                    Some('\0') => {
                        // This is an unexpected-null-character parse error.
                        // Append a U+FFFD REPLACEMENT CHARACTER character to the current attribute's value.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                            && let Some(attr) = tag.attributes.last().as_mut()
                        {
                            attr.value.push('\u{FFFD}');
                        }
                    }
                    None => {
                        // This is an eof-in-tag parse error.
                        // Emit an end-of-file token.
                        return Token::EOF;
                    }
                    Some(ch) => {
                        // Append the current input character to the current attribute's value.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                            && let Some(attr) = tag.attributes.last().as_mut()
                        {
                            attr.value.push(ch);
                        }
                    }
                },
                // https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
                State::SelfClosingStartTag => match c {
                    Some('>') => {
                        // Set the self-closing flag of the current tag token.
                        // Switch to the data state.
                        // Emit the current tag token.
                        if let Some(t) = self.current_token.as_mut()
                            && let Token::StartTag(tag) = t
                        {
                            tag.self_closing = true;
                        }
                        self.state = State::Data;
                        return self.emit_tag_token();
                    }
                    None => {
                        // This is an eof-in-tag parse error.
                        // Emit an end-of-file token.
                        return Token::EOF;
                    }
                    _ => {
                        // This is an unexpected-solidus-in-tag parse error.
                        // Reconsume in the before attribute name state.
                        self.state = State::BeforeAttributeName;
                        self.reconsume_char = c;
                    }
                },
            }
        }
    }

    fn emit_tag_token(&mut self) -> Token {
        self.current_token
            .take()
            .expect("tokenizer.current_tag is invalid")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
