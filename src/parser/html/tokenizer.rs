use std::{iter::Peekable, str::Chars};

use crate::dom::{Atom, Attr};

pub(crate) enum ProcessResult {
    Continue,
    Reconsume(State),
    ReconsumeAndEmitToken(State, Token),
    ReconsumeAndEmitTokens(State, Vec<Token>),
    Switch(State),
    SwitchAndEmitToken(State, Token),
    EmitEOF,
    EmitChar(char),
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
    RCData,
    RCDataLessThanSign,
    RCDataEndTagOpen,
    RCDataEndTagName,
    RawText,
    RawTextLessThanSign,
    RawTextEndTagOpen,
    RawTextEndTagName,
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
    temp_buf: Vec<char>,
    last_start_tag_name: Option<Atom>,
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
            temp_buf: Vec::new(),
            last_start_tag_name: None,
        }
    }

    pub(crate) fn switch(&mut self, state: State) {
        self.state = state;
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
                    self.switch(state);
                }
                ProcessResult::ReconsumeAndEmitToken(state, token) => {
                    self.switch(state);
                    return token;
                }
                ProcessResult::ReconsumeAndEmitTokens(state, mut tokens) => {
                    self.switch(state);
                    let token = tokens
                        .pop()
                        .expect("[Tokenizer] tokens should not be empty");
                    self.pending_tokens = tokens;
                    return token;
                }
                ProcessResult::Switch(state) => {
                    self.input.next();
                    self.switch(state);
                }
                ProcessResult::SwitchAndEmitToken(state, token) => {
                    self.input.next();
                    self.switch(state);
                    return token;
                }
                ProcessResult::EmitEOF => {
                    return Token::EOF;
                }
                ProcessResult::EmitChar(ch) => {
                    self.input.next();
                    return Token::Char(ch);
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
            State::RCData => self.handle_rcdata(c),
            State::RCDataLessThanSign => self.handle_rcdata_less_than_sign(c),
            State::RCDataEndTagOpen => self.handle_rcdata_end_tag_open(c),
            State::RCDataEndTagName => self.handle_rcdata_end_tag_name(c),
            State::RawText => self.handle_raw_text(c),
            State::RawTextLessThanSign => self.handle_raw_text_less_than_sign(c),
            State::RawTextEndTagOpen => self.handle_raw_text_end_tag_open(c),
            State::RawTextEndTagName => self.handle_raw_text_end_tag_name(c),
        }
    }

    fn convert_temp_buf_to_tokens(&self) -> Vec<Token> {
        let mut tokens = Vec::with_capacity(self.temp_buf.len() + 2);
        self.temp_buf.iter().rev().for_each(|ch| {
            tokens.push(Token::Char(*ch));
        });
        tokens.push(Token::Char('/'));
        tokens.push(Token::Char('<'));
        tokens
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-end-tag-name-state
    fn handle_raw_text_end_tag_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => {
                if self.is_appropriate_end_tag() {
                    ProcessResult::Switch(State::BeforeAttrName)
                } else {
                    ProcessResult::ReconsumeAndEmitTokens(
                        State::RawText,
                        self.convert_temp_buf_to_tokens(),
                    )
                }
            }
            Some(ch) if ch == '/' => {
                if self.is_appropriate_end_tag() {
                    ProcessResult::Switch(State::SelfClosingStartTag)
                } else {
                    ProcessResult::ReconsumeAndEmitTokens(
                        State::RawText,
                        self.convert_temp_buf_to_tokens(),
                    )
                }
            }
            Some(ch) if ch == '>' => {
                if self.is_appropriate_end_tag() {
                    ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token())
                } else {
                    ProcessResult::ReconsumeAndEmitTokens(
                        State::RawText,
                        self.convert_temp_buf_to_tokens(),
                    )
                }
            }
            Some(ch) if ch.is_ascii_alphabetic() => {
                self.cur_tag_name.push(ch.to_ascii_lowercase());
                self.temp_buf.push(ch);
                ProcessResult::Continue
            }
            _ => ProcessResult::ReconsumeAndEmitTokens(
                State::RawText,
                self.convert_temp_buf_to_tokens(),
            ),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-end-tag-open-state
    fn handle_raw_text_end_tag_open(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if ch.is_ascii_alphabetic() => {
                self.create_end_tag();
                ProcessResult::Reconsume(State::RawTextEndTagName)
            }
            _ => ProcessResult::ReconsumeAndEmitTokens(
                State::RawText,
                vec![Token::Char('/'), Token::Char('<')],
            ),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-less-than-sign-state
    fn handle_raw_text_less_than_sign(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some('/') => {
                self.temp_buf.clear();
                ProcessResult::Switch(State::RawTextEndTagOpen)
            }
            _ => ProcessResult::ReconsumeAndEmitToken(State::RawText, Token::Char('<')),
        }
    }

    fn is_whitespace(&self, ch: char) -> bool {
        ch == '\t' || ch == '\n' || ch == '\x0C' || ch == ' '
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rcdata-end-tag-name-state
    fn handle_rcdata_end_tag_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => {
                if self.is_appropriate_end_tag() {
                    ProcessResult::Switch(State::BeforeAttrName)
                } else {
                    ProcessResult::ReconsumeAndEmitTokens(
                        State::RCData,
                        self.convert_temp_buf_to_tokens(),
                    )
                }
            }
            Some(ch) if ch == '/' => {
                if self.is_appropriate_end_tag() {
                    ProcessResult::Switch(State::SelfClosingStartTag)
                } else {
                    ProcessResult::ReconsumeAndEmitTokens(
                        State::RCData,
                        self.convert_temp_buf_to_tokens(),
                    )
                }
            }
            Some(ch) if ch == '>' => {
                if self.is_appropriate_end_tag() {
                    ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token())
                } else {
                    ProcessResult::ReconsumeAndEmitTokens(
                        State::RCData,
                        self.convert_temp_buf_to_tokens(),
                    )
                }
            }
            Some(ch) if ch.is_ascii_alphabetic() => {
                self.cur_tag_name.push(ch.to_ascii_lowercase());
                self.temp_buf.push(ch);
                ProcessResult::Continue
            }
            _ => ProcessResult::ReconsumeAndEmitTokens(
                State::RCData,
                self.convert_temp_buf_to_tokens(),
            ),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#appropriate-end-tag-token
    fn is_appropriate_end_tag(&self) -> bool {
        match &self.last_start_tag_name {
            Some(last)
                if last.as_ref() == self.cur_tag_name && self.cur_tag_type == TagType::End =>
            {
                true
            }
            _ => false,
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rcdata-end-tag-open-state
    fn handle_rcdata_end_tag_open(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if ch.is_ascii_alphabetic() => {
                self.create_end_tag();
                ProcessResult::Reconsume(State::RCDataEndTagName)
            }
            _ => ProcessResult::ReconsumeAndEmitTokens(
                State::RCData,
                vec![Token::Char('/'), Token::Char('<')],
            ),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rcdata-less-than-sign-state
    fn handle_rcdata_less_than_sign(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some('/') => {
                self.temp_buf.clear();
                ProcessResult::Switch(State::RCDataEndTagOpen)
            }
            _ => ProcessResult::ReconsumeAndEmitToken(State::RCData, Token::Char('<')),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rcdata-state
    fn handle_rcdata(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '<' => ProcessResult::Switch(State::RCDataLessThanSign),
                _ => ProcessResult::EmitChar(ch),
            },
            None => ProcessResult::EmitEOF,
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#rawtext-state
    fn handle_raw_text(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) => match ch {
                '<' => ProcessResult::Switch(State::RawTextLessThanSign),
                _ => ProcessResult::EmitChar(ch),
            },
            None => ProcessResult::EmitEOF,
        }
    }

    /// TODO: comment token
    /// https://html.spec.whatwg.org/multipage/parsing.html#bogus-comment-state
    /// https://html.spec.whatwg.org/multipage/parsing.html#markup-declaration-open-state
    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-start-state
    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-state
    /// https://html.spec.whatwg.org/multipage/parsing.html#comment-end-state
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

    /// https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
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

    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-value-(quoted)-state
    fn handle_after_quoted_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => ProcessResult::Switch(State::BeforeAttrName),
            Some('/') => ProcessResult::Switch(State::SelfClosingStartTag),
            Some('>') => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
            Some(_) => {
                self.print_parse_error("missing-whitespace-between-attributes");
                ProcessResult::Reconsume(State::BeforeAttrName)
            }
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(unquoted)-state
    fn handle_unquoted_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => ProcessResult::Switch(State::BeforeAttrName),
            Some('>') => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
            Some(ch) if ch == '"' || ch == '\'' || ch == '<' || ch == '=' || ch == '`' => {
                self.print_parse_error("unexpected-character-in-unquoted-attribute-value");
                self.cur_attr_value.push(ch);
                ProcessResult::Continue
            }
            Some(ch) => {
                self.cur_attr_value.push(ch);
                ProcessResult::Continue
            }
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(single-quoted)-state
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

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
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

    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
    fn handle_before_attr_value(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => ProcessResult::Continue,
            Some('"') => ProcessResult::Switch(State::DoubleQuotedAttrValue),
            Some('\'') => ProcessResult::Switch(State::SingleQuotedAttrValue),
            Some('>') => {
                self.print_parse_error("missing-attribute-value");
                ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token())
            }
            Some(_) => ProcessResult::Reconsume(State::UnquotedAttrValue),
            None => ProcessResult::Reconsume(State::UnquotedAttrValue),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
    fn handle_after_attr_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => ProcessResult::Continue,
            Some('/') => ProcessResult::Switch(State::SelfClosingStartTag),
            Some('=') => ProcessResult::Switch(State::BeforeAttrValue),
            Some('>') => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
            Some(_) => {
                self.create_attr();
                ProcessResult::Reconsume(State::AttrName)
            }
            None => {
                self.print_parse_error("eof-in-tag");
                ProcessResult::EmitEOF
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
    fn handle_attr_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) || ch == '/' || ch == '>' => {
                ProcessResult::Reconsume(State::AfterAttrName)
            }
            Some('=') => ProcessResult::Switch(State::BeforeAttrValue),
            Some(ch) if ch == '"' || ch == '\'' || ch == '<' => {
                self.print_parse_error("unexpected-character-in-attribute-name");
                self.cur_attr_name.push(ch.to_ascii_lowercase());
                ProcessResult::Continue
            }
            Some(ch) => {
                self.cur_attr_name.push(ch.to_ascii_lowercase());
                ProcessResult::Continue
            }
            None => ProcessResult::Reconsume(State::AfterAttrName),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
    fn handle_before_attr_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => ProcessResult::Continue,
            Some('/') | Some('>') => ProcessResult::Reconsume(State::AfterAttrName),
            Some('=') => {
                self.print_parse_error("unexpected-equals-sign-before-attribute-name");
                self.create_attr();
                self.cur_attr_name.push('=');
                ProcessResult::Switch(State::AttrName)
            }
            Some(_) => {
                self.create_attr();
                ProcessResult::Reconsume(State::AttrName)
            }
            None => ProcessResult::Reconsume(State::AfterAttrName),
        }
    }

    fn create_attr(&mut self) {
        self.append_attr();
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
    fn handle_tag_name(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some(ch) if self.is_whitespace(ch) => ProcessResult::Switch(State::BeforeAttrName),
            Some('/') => ProcessResult::Switch(State::SelfClosingStartTag),
            Some('>') => ProcessResult::SwitchAndEmitToken(State::Data, self.cur_tag_token()),
            Some(ch) => {
                self.cur_tag_name.push(ch.to_ascii_lowercase());
                ProcessResult::Continue
            }
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

        let token = match self.cur_tag_type {
            TagType::Start => {
                self.last_start_tag_name = Some(Atom::from(&*self.cur_tag_name));
                Token::StartTag(tag)
            }
            TagType::End => Token::EndTag(tag),
        };

        self.cur_tag_name.clear();

        token
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
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

    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
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

    /// https://html.spec.whatwg.org/multipage/parsing.html#data-state
    fn handle_data(&mut self, c: Option<char>) -> ProcessResult {
        match c {
            Some('<') => ProcessResult::Switch(State::TagOpen),
            Some(ch) => ProcessResult::EmitChar(ch),
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
