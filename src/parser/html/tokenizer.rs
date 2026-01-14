use std::{iter::Peekable, str::Chars};

use crate::dom::{Atom, Attr};

pub(crate) enum ProcessResult {
    Continue,
    Reconsume(State),
    ReconsumeAndEmitToken(State, Token),
    Switch(State),
    SwitchAndEmitToken(State, Token),
    EmitEOF,
    EmitToken(Token),
    EmitTokens(Vec<Token>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TagType {
    Start,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tag {
    pub(crate) name: Atom,
    pub(crate) self_closing: bool,
    pub(crate) attrs: Vec<Attr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    EOF,
    Char(char),
    StartTag(Tag),
    EndTag(Tag),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum State {
    Data,
    TagOpen,
    TagName,
    BeforeAttrName,
    AttrName,
    AfterAttrName,
    BeforeAttrValue,
    UnquotedAttrValue,
    DoubleQuotedAttrValue,
    SingleQuotedAttrValue,
    AfterQuotedAttrValue,
    SelfClosingStartTag,
    EndTagOpen,
    Comment,
}

pub(crate) struct Tokenizer<'a> {
    input: Peekable<Chars<'a>>,
    state: State,
    pending_tokens: Vec<Token>,
    cur_tag_type: TagType,
    cur_tag_name: String,
    cur_tag_self_closing: bool,
    cur_tag_attrs: Vec<Attr>,
    cur_attr_name: String,
    cur_attr_value: String,
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(input: &'a str) -> Tokenizer<'a> {
        Tokenizer {
            input: input.chars().peekable(),
            state: State::Data,
            pending_tokens: Vec::new(),
            cur_tag_type: TagType::Start,
            cur_tag_name: String::new(),
            cur_tag_self_closing: false,
            cur_tag_attrs: Vec::new(),
            cur_attr_name: String::new(),
            cur_attr_value: String::new(),
        }
    }

    pub(crate) fn next(&mut self) -> Token {
        if let Some(token) = self.pending_tokens.pop() {
            return token;
        }

        loop {
            let c = self.input.peek().copied();

            match self.process(c) {
                ProcessResult::Continue => {
                    self.input.next();
                }
                ProcessResult::Reconsume(state) => {
                    self.state = state;
                }
                ProcessResult::ReconsumeAndEmitToken(state, token) => {
                    self.state = state;
                    return token;
                }
                ProcessResult::Switch(state) => {
                    self.input.next();
                    self.state = state;
                }
                ProcessResult::SwitchAndEmitToken(state, token) => {
                    self.input.next();
                    self.state = state;
                    return token;
                }
                ProcessResult::EmitEOF => {
                    return Token::EOF;
                }
                ProcessResult::EmitToken(token) => {
                    self.input.next();
                    return token;
                }
                ProcessResult::EmitTokens(mut tokens) => {
                    self.input.next();
                    let token = tokens
                        .pop()
                        .expect("[Tokenizer] tokens should not be empty");
                    self.pending_tokens = tokens;
                    return token;
                }
            }
        }
    }

    fn process(&mut self, c: Option<char>) -> ProcessResult {
        match self.state {
            State::Data => self.handle_data(c),
            State::TagOpen => self.handle_tag_open(c),
            State::EndTagOpen => self.handle_end_tag_open(c),
            State::TagName => self.handle_tag_name(c),
            State::BeforeAttrName => self.handle_before_attr_name(c),
            State::AttrName => self.handle_attr_name(c),
            State::AfterAttrName => self.handle_after_attr_name(c),
            State::BeforeAttrValue => self.handle_before_attr_value(c),
            State::DoubleQuotedAttrValue => self.handle_double_quoted_attr_value(c),
            State::SingleQuotedAttrValue => self.handle_single_quoted_attr_value(c),
            State::UnquotedAttrValue => self.handle_unquoted_attr_value(c),
            State::AfterQuotedAttrValue => self.handle_after_quoted_attr_value(c),
            State::SelfClosingStartTag => self.handle_self_closing_start_tag(c),
            State::Comment => self.handle_comment(c),
        }
    }

    fn handle_comment(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '>' => ProcessResult::Switch(State::Data),
                _ => ProcessResult::Continue,
            },
            None => {
                self.print_parse_error("eof-in-comment");
                ProcessResult::EmitEOF
            }
        }
    }

    fn handle_self_closing_start_tag(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '>' => {
                    self.cur_tag_self_closing = true;
                    ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token())
                }
                _ => {
                    self.print_parse_error("unexpected-solidus-in-tag");
                    ProcessResult::Reconsume(State::BeforeAttrName)
                }
            },
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    fn handle_after_quoted_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Switch(State::BeforeAttrName),
                '/' => ProcessResult::Switch(State::SelfClosingStartTag),
                '>' => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
                _ => {
                    self.print_parse_error("missing-whitespace-between-attributes");
                    ProcessResult::Reconsume(State::BeforeAttrName)
                }
            },
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    fn handle_unquoted_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Switch(State::BeforeAttrName),
                '>' => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
                '"' | '\'' | '<' | '=' | '`' => {
                    self.print_parse_error("unexpected-character-in-unquoted-attribute-value");
                    self.cur_attr_value.push(ch);
                    ProcessResult::Continue
                }
                _ => {
                    self.cur_attr_value.push(ch);
                    ProcessResult::Continue
                }
            },
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    fn handle_single_quoted_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\'' => ProcessResult::Switch(State::AfterQuotedAttrValue),
                _ => {
                    self.cur_attr_value.push(ch);
                    ProcessResult::Continue
                }
            },
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    fn handle_double_quoted_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '"' => ProcessResult::Switch(State::AfterQuotedAttrValue),
                _ => {
                    self.cur_attr_value.push(ch);
                    ProcessResult::Continue
                }
            },
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    fn handle_before_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Continue,
                '"' => ProcessResult::Switch(State::DoubleQuotedAttrValue),
                '\'' => ProcessResult::Switch(State::SingleQuotedAttrValue),
                '>' => {
                    self.print_parse_error("missing-attribute-value");
                    ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token())
                }
                _ => ProcessResult::Reconsume(State::UnquotedAttrValue),
            },
            None => ProcessResult::Reconsume(State::UnquotedAttrValue),
        }
    }

    fn handle_after_attr_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Continue,
                '/' => ProcessResult::Switch(State::SelfClosingStartTag),
                '=' => ProcessResult::Switch(State::BeforeAttrValue),
                '>' => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
                _ => {
                    self.create_attr();
                    ProcessResult::Reconsume(State::AttrName)
                }
            },
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    fn handle_attr_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                    ProcessResult::Reconsume(State::AfterAttrName)
                }
                '=' => ProcessResult::Switch(State::BeforeAttrValue),
                '"' | '\'' | '<' => {
                    self.print_parse_error("unexpected-character-in-attribute-name");
                    self.cur_attr_name.push(ch.to_ascii_lowercase());
                    ProcessResult::Continue
                }
                _ => {
                    self.cur_attr_name.push(ch.to_ascii_lowercase());
                    ProcessResult::Continue
                }
            },
            None => ProcessResult::Reconsume(State::AfterAttrName),
        }
    }

    fn handle_before_attr_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Continue,
                '/' | '>' => ProcessResult::Reconsume(State::AfterAttrName),
                '=' => {
                    self.print_parse_error("unexpected-equals-sign-before-attribute-name");
                    self.create_attr();
                    self.cur_attr_name.push(ch);
                    ProcessResult::Switch(State::AttrName)
                }
                _ => {
                    self.create_attr();
                    ProcessResult::Reconsume(State::AttrName)
                }
            },
            None => ProcessResult::Reconsume(State::AfterAttrName),
        }
    }

    fn create_attr(&mut self) {
        self.append_attr();
    }

    fn handle_tag_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '\t' | '\n' | '\x0C' | ' ' => ProcessResult::Switch(State::BeforeAttrName),
                '/' => ProcessResult::Switch(State::SelfClosingStartTag),
                '>' => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
                _ => {
                    self.cur_tag_name.push(ch.to_ascii_lowercase());
                    ProcessResult::Continue
                }
            },
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    fn append_attr(&mut self) {
        if self.cur_attr_name.is_empty() {
            self.cur_attr_value.clear();
            return;
        }

        self.cur_tag_attrs.push(Attr {
            name: Atom::from(&*self.cur_attr_name),
            value: std::mem::take(&mut self.cur_attr_value),
        });

        self.cur_attr_name.clear();
    }

    fn cur_tag_token(&mut self) -> Token {
        self.append_attr();

        let tag = Tag {
            name: Atom::from(&*self.cur_tag_name),
            self_closing: self.cur_tag_self_closing,
            attrs: std::mem::take(&mut self.cur_tag_attrs),
        };

        self.cur_tag_name.clear();

        match self.cur_tag_type {
            TagType::Start => Token::StartTag(tag),
            TagType::End => Token::EndTag(tag),
        }
    }

    fn handle_end_tag_open(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                ch if ch.is_ascii_alphabetic() => {
                    self.create_end_tag();
                    ProcessResult::Reconsume(State::TagName)
                }
                '>' => {
                    self.print_parse_error("missing-end-tag-name");
                    ProcessResult::Switch(State::Data)
                }
                _ => {
                    self.print_parse_error("invalid-first-character-of-tag-name");
                    ProcessResult::Switch(State::Comment)
                }
            },
            None => {
                self.print_parse_error("eof-before-tag-name");
                ProcessResult::EmitTokens(vec![Token::EOF, Token::Char('/'), Token::Char('<')])
            }
        }
    }

    fn handle_tag_open(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '!' => ProcessResult::Switch(State::Comment),
                '/' => ProcessResult::Switch(State::EndTagOpen),
                ch if ch.is_ascii_alphabetic() => {
                    self.create_start_tag();
                    ProcessResult::Reconsume(State::TagName)
                }
                '?' => {
                    self.print_parse_error("unexpected-question-mark-instead-of-tag-name");
                    ProcessResult::Switch(State::Comment)
                }
                _ => {
                    self.print_parse_error("invalid-first-character-of-tag-name");
                    ProcessResult::ReconsumeAndEmitToken(State::Data, Token::Char('<'))
                }
            },
            None => {
                self.print_parse_error("eof-before-tag-name");
                ProcessResult::EmitTokens(vec![Token::EOF, Token::Char('<')])
            }
        }
    }

    fn create_end_tag(&mut self) {
        self.cur_tag_type = TagType::End;
        self.create_tag();
    }

    fn create_start_tag(&mut self) {
        self.cur_tag_type = TagType::Start;
        self.create_tag();
    }

    fn create_tag(&mut self) {
        self.cur_tag_name.clear();
        self.cur_tag_self_closing = false;
        self.cur_tag_attrs.clear();
        self.clear_attr();
    }

    fn clear_attr(&mut self) {
        self.cur_attr_name.clear();
        self.cur_attr_value.clear();
    }

    fn handle_data(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some('<') => ProcessResult::Switch(State::TagOpen),
            Some(ch) => ProcessResult::EmitToken(Token::Char(ch)),
            None => ProcessResult::EmitEOF,
        }
    }

    fn print_parse_error(&self, err: &str) {
        println!("[Tokenizer] Parse error: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_tokens(input: &str) -> Vec<Token> {
        let mut tokenizer = Tokenizer::new(input);
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

    fn attr(name: &str, value: &str) -> Attr {
        Attr {
            name: Atom::from(name),
            value: value.to_string(),
        }
    }

    fn start_tag(name: &str, attributes: Vec<Attr>, self_closing: bool) -> Token {
        Token::StartTag(Tag {
            name: Atom::from(name),
            self_closing,
            attrs: attributes,
        })
    }

    fn end_tag(name: &str) -> Token {
        Token::EndTag(Tag {
            name: Atom::from(name),
            self_closing: false,
            attrs: Vec::new(),
        })
    }

    #[test]
    fn test_basic_text() {
        let tokens = collect_tokens("abc");
        assert_eq!(
            tokens,
            vec![
                Token::Char('a'),
                Token::Char('b'),
                Token::Char('c'),
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
        let tokens = collect_tokens("<DIV></div >");
        assert_eq!(
            tokens,
            vec![start_tag("div", vec![], false), end_tag("div"), Token::EOF]
        );
    }

    #[test]
    fn test_attributes_mixed() {
        let tokens = collect_tokens("<div id=\"test\" class=foo checked>");

        let expected_attrs = vec![
            attr("id", "test"),
            attr("class", "foo"),
            attr("checked", ""),
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
    fn test_eof_in_tag_edge_case() {
        let tokens = collect_tokens("</");
        assert_eq!(tokens, vec![Token::Char('<'), Token::Char('/'), Token::EOF]);
    }

    #[test]
    fn test_invalid_tag_name_start() {
        let tokens = collect_tokens("<4");
        assert_eq!(tokens, vec![Token::Char('<'), Token::Char('4'), Token::EOF]);
    }

    #[test]
    fn test_attribute_value_with_illegal_chars() {
        let tokens = collect_tokens("<div data=foo\"bar>");

        assert_eq!(
            tokens[0],
            start_tag("div", vec![attr("data", "foo\"bar")], false)
        );
    }

    #[test]
    fn test_unexpected_equals_sign_before_attribute_name() {
        let tokens = collect_tokens("<div =foo>");
        assert_eq!(tokens[0], start_tag("div", vec![attr("=foo", "")], false));
    }
}
