use std::borrow::Cow;

pub(crate) enum Token<'a> {
    Whitespace(&'a str),
    String(&'a str),
    BadString(&'a str),
    EOF,
}

/// https://drafts.csswg.org/css-syntax/#tokenization
pub(crate) struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(input: &'a str) -> Tokenizer<'a> {
        Tokenizer { input, pos: 0 }
    }

    fn has_at_least(&self, n: usize) -> bool {
        self.pos + n < self.input.len()
    }

    fn is_eof(&self) -> bool {
        !self.has_at_least(0)
    }

    fn byte_at(&self, offset: usize) -> u8 {
        self.input.as_bytes()[self.pos + offset]
    }

    fn next_byte_unchecked(&self) -> u8 {
        self.byte_at(0)
    }

    fn next_byte(&self) -> Option<u8> {
        if self.is_eof() {
            None
        } else {
            Some(self.byte_at(0))
        }
    }

    fn consume_newline(&mut self) {
        let b = self.next_byte_unchecked();

        self.advance(1);

        if b == b'\r' && self.next_byte() == Some(b'\n') {
            self.advance(1);
        }
    }

    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    fn slice(&self, start: usize, end: usize) -> &'a str {
        unsafe { self.input.get_unchecked(start..end) }
    }

    fn slice_from(&self, start_pos: usize) -> &'a str {
        self.slice(start_pos, self.pos)
    }

    #[inline]
    fn print_parse_error(&self, err: &str) {
        println!("[CSS Tokenizer] Parse error: {}", err);
    }

    /// https://drafts.csswg.org/css-syntax/#consume-token
    pub(crate) fn next(&mut self) -> Token<'a> {
        if self.is_eof() {
            return Token::EOF;
        }

        let b = self.next_byte_unchecked();

        match b {
            b' ' | b'\t' => self.consume_whitespace(false),
            b'\n' | b'\x0C' | b'\r' => self.consume_whitespace(true),
            b'"' => self.consume_string(false),
            b'\'' => self.consume_string(true),
            _ => todo!(),
        }
    }

    fn next_char(&self) -> char {
        unsafe { self.input.get_unchecked(self.pos..) }
            .chars()
            .next()
            .expect("[CSS Tokenizer] next_char shoud not be empty")
    }

    fn consume_char(&mut self) -> char {
        let c = self.next_char();
        self.advance(c.len_utf8());
        c
    }

    fn consume_hex_digits(&mut self) -> char {
        
    }

    /// https://drafts.csswg.org/css-syntax/#consume-an-escaped-code-point
    fn consume_escaped_code_point(&mut self) -> char {
        if self.is_eof() {
            return '\u{FFFD}';
        }

        match self.next_byte_unchecked() {
            b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => {
                let c = self.consume_hex_digits();
                '\u{FFFD}'
            }
            b'\0' => {
                self.advance(1);
                '\u{FFFD}'
            }
            _ => self.consume_char(),
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-string-token
    fn consume_string(&mut self, single_quote: bool) -> Token<'a> {
        self.advance(1);

        let start_pos = self.pos;

        loop {
            if self.is_eof() {
                self.print_parse_error("consume_string EOF");
                return Token::String(self.slice_from(start_pos));
            }

            match self.next_byte_unchecked() {
                b'\'' => {
                    if single_quote {
                        let res = Token::String(self.slice_from(start_pos));
                        self.advance(1);
                        return res;
                    }
                    self.advance(1);
                }
                b'"' => {
                    if !single_quote {
                        let res = Token::String(self.slice_from(start_pos));
                        self.advance(1);
                        return res;
                    }
                    self.advance(1);
                }
                b'\n' | b'\r' | b'\x0C' => {
                    return Token::BadString(self.slice_from(start_pos));
                }
                b'\\' => {
                    let tmp = self.slice_from(start_pos);

                    self.advance(1);

                    if self.is_eof() {
                        return Token::String(tmp);
                    }

                    match self.next_byte_unchecked() {
                        b'\n' | b'\r' | b'\x0C' => {
                            self.consume_newline();
                        }
                        _ => {
                            self.consume_escaped_code_point();
                        }
                    }
                }
                b'\0' => {
                    self.advance(1);
                    // TODO: \u{FFFD}
                }
                _ => {
                    self.advance(1);
                }
            }
        }
    }

    /// https://drafts.csswg.org/css-syntax/#whitespace-token-diagram
    fn consume_whitespace(&mut self, newline: bool) -> Token<'a> {
        let start_pos = self.pos;

        if newline {
            self.consume_newline();
        } else {
            self.advance(1);
        }

        while !self.is_eof() {
            let b = self.next_byte_unchecked();

            match b {
                b' ' | b'\t' => self.advance(1),
                b'\n' | b'\x0C' | b'\r' => self.consume_newline(),
                _ => break,
            }
        }

        Token::Whitespace(self.slice_from(start_pos))
    }
}
