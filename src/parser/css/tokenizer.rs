use std::borrow::Cow;

pub(crate) enum NumberType {
    Integer,
    Number,
}

pub(crate) enum Token<'a> {
    Whitespace(&'a str),
    String(Cow<'a, str>),
    BadString(Cow<'a, str>),
    Hash(Cow<'a, str>),
    IDHash(Cow<'a, str>),
    Ident(Cow<'a, str>),
    Function(Cow<'a, str>),
    UnquotedURL(Cow<'a, str>),
    BadURL(Cow<'a, str>),
    SuffixMatch,
    Parenthesis,
    CloasParenthesis,
    SubstringMatch,
    Percentage(f64, Option<char>),
    Number(f64, NumberType, Option<char>),
    Dimension(f64, NumberType, Option<char>, Cow<'a, str>),
    Delim(char),
    Comma,
    CDC,
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

    fn start_with(&self, needle: &[u8]) -> bool {
        self.input.as_bytes()[self.pos..].starts_with(needle)
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
            b'#' => self.consume_hash(),
            b'$' => {
                if self.start_with(b"$=") {
                    self.advance(2);
                    Token::SuffixMatch
                } else {
                    self.advance(1);
                    Token::Delim('$')
                }
            }
            b'(' => {
                self.advance(1);
                Token::Parenthesis
            }
            b')' => {
                self.advance(1);
                Token::CloasParenthesis
            }
            b'*' => {
                if self.start_with(b"*=") {
                    self.advance(2);
                    Token::SubstringMatch
                } else {
                    self.advance(1);
                    Token::Delim('*')
                }
            }
            b'+' => {
                if (self.has_at_least(1) && self.byte_at(1).is_ascii_digit())
                    || (self.has_at_least(2)
                        && self.byte_at(1) == b'.'
                        && self.byte_at(2).is_ascii_digit())
                {
                    self.consume_numeric()
                } else {
                    self.advance(1);
                    Token::Delim('+')
                }
            }
            b',' => {
                self.advance(1);
                Token::Comma
            }
            b'-' => {
                if (self.has_at_least(1) && self.byte_at(1).is_ascii_digit())
                    || (self.has_at_least(2)
                        && self.byte_at(1) == b'.'
                        && self.byte_at(2).is_ascii_digit())
                {
                    self.consume_numeric()
                } else if self.start_with(b"-->") {
                    self.advance(3);
                    Token::CDC
                } else if self.is_ident_start() {
                    self.consume_ident_like()
                } else {
                    self.advance(1);
                    Token::Delim('-')
                }
            }
            _ => todo!(),
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-ident-like-token
    fn consume_ident_like(&mut self) -> Token<'a> {
        let value = self.consume_name();

        if !self.is_eof() && self.next_byte_unchecked() == b'(' {
            self.advance(1);
            if value.eq_ignore_ascii_case("url") {
                self.consume_unquoted_url()
                    .unwrap_or(Token::Function(value))
            } else {
                Token::Function(value)
            }
        } else {
            Token::Ident(value)
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-url-token
    fn consume_unquoted_url(&mut self) -> Result<Token<'a>, ()> {
        let start_pos = self.pos;
        let from_start = &self.input[self.pos..];
        let mut found_printable_char = false;
        let mut iter = from_start.bytes().enumerate();

        loop {
            let (offset, b) = match iter.next() {
                Some(i) => i,
                None => {
                    self.pos = self.input.len();
                    break;
                }
            };

            match b {
                b' ' | b'\t' | b'\n' | b'\x0C' | b'\r' => {}
                b'"' | b'\'' => return Err(()),
                b')' => {
                    self.advance(offset + 1);
                    break;
                }
                _ => {
                    self.advance(offset);
                    found_printable_char = true;
                    break;
                }
            }
        }

        if found_printable_char {
            Ok(self.consume_unquoted_url_internal())
        } else {
            Ok(Token::UnquotedURL("".into()))
        }
    }

    fn consume_unquoted_url_internal(&mut self) -> Token<'a> {
        let start_pos = self.pos;
        let mut str_bytes: Vec<u8>;
        loop {
            if self.is_eof() {
                return Token::UnquotedURL(self.slice_from(start_pos).into());
            }

            match self.next_byte_unchecked() {
                b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => {
                    return self.consume_url_end(start_pos, self.slice_from(start_pos).into());
                }
                b')' => {
                    let value = self.slice_from(start_pos);
                    self.advance(1);
                    return Token::UnquotedURL(value.into());
                }
                b'\x01'..=b'\x08' | b'\x0B' | b'\x0E'..=b'\x1F' | b'\x7F' | b'"' | b'\'' | b'(' => {
                    self.advance(1);
                    return self.consume_bad_url(start_pos);
                }
                b'\\' | b'\0' => {
                    str_bytes = self.slice_from(start_pos).as_bytes().to_owned();
                    break;
                }
                _ => {
                    self.advance(1);
                }
            }
        }

        while !self.is_eof() {
            let b = self.next_byte_unchecked();

            match b {
                b' ' | b'\t' | b'\n' | b'\r' | b'\x0C' => {
                    let s = unsafe { String::from_utf8_unchecked(str_bytes) }.into();
                    return self.consume_url_end(start_pos, s);
                }
                b')' => {
                    self.advance(1);
                    break;
                }
                b'\x01'..=b'\x08' | b'\x0B' | b'\x0E'..=b'\x1F' | b'\x7F' | b'"' | b'\'' | b'(' => {
                    self.advance(1);
                    return self.consume_bad_url(start_pos);
                }
                b'\\' => {
                    self.advance(1);
                    if self.has_newline_at(0) {
                        return self.consume_bad_url(start_pos);
                    }

                    let c = self.consume_escaped_code_point();
                    str_bytes.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
                }
                b'\0' => {
                    self.advance(1);
                    str_bytes.extend("\u{FFFD}".as_bytes());
                }
                b => {
                    self.advance(1);
                    str_bytes.push(b);
                }
            }
        }

        Token::UnquotedURL(unsafe { String::from_utf8_unchecked(str_bytes) }.into())
    }

    /// https://drafts.csswg.org/css-syntax/#consume-the-remnants-of-a-bad-url
    fn consume_bad_url(&mut self, start_pos: usize) -> Token<'a> {
        while !self.is_eof() {
            match self.next_byte_unchecked() {
                b')' => {
                    let value = self.slice_from(start_pos).into();
                    self.advance(1);
                    return Token::BadURL(value);
                }
                b'\\' => {
                    self.advance(1);
                    if matches!(self.next_byte(), Some(b')') | Some(b'\\')) {
                        self.advance(1);
                    }
                }
                b'\n' | b'\x0C' | b'\r' => {
                    self.consume_newline();
                }
                _ => {
                    self.advance(1);
                }
            }
        }
        Token::BadURL(self.slice_from(start_pos).into())
    }

    fn consume_url_end(&mut self, start_pos: usize, s: Cow<'a, str>) -> Token<'a> {
        while !self.is_eof() {
            match self.next_byte_unchecked() {
                b')' => {
                    self.advance(1);
                    break;
                }
                b' ' | b'\t' => {
                    self.advance(1);
                }
                b'\n' | b'\x0C' | b'\r' => {
                    self.consume_newline();
                }
                _ => {
                    self.advance(1);
                    return self.consume_bad_url(start_pos);
                }
            }
        }
        Token::UnquotedURL(s)
    }

    /// https://drafts.csswg.org/css-syntax/#consume-numeric-token
    fn consume_numeric(&mut self) -> Token<'a> {
        let (value, num_type, sign) = self.consume_number();

        if !self.is_eof() && self.next_byte_unchecked() == b'%' {
            self.advance(1);
            Token::Percentage(value, sign)
        } else if self.is_ident_start() {
            let unit = self.consume_name();
            Token::Dimension(value, num_type, sign, unit)
        } else {
            Token::Number(value, num_type, sign)
        }
    }

    fn byte_to_decimal_digit(&self, b: u8) -> Option<u32> {
        if b.is_ascii_digit() {
            Some((b - b'0') as u32)
        } else {
            None
        }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-number
    fn consume_number(&mut self) -> (f64, NumberType, Option<char>) {
        let mut num_type = NumberType::Integer;

        let sign = match self.next_byte_unchecked() {
            b'+' => {
                self.advance(1);
                Some('+')
            }
            b'-' => {
                self.advance(1);
                Some('-')
            }
            _ => None,
        };

        let mut int_part = 0.;

        while let Some(digit) = self.byte_to_decimal_digit(self.next_byte_unchecked()) {
            int_part = int_part * 10. + digit as f64;
            self.advance(1);
            if self.is_eof() {
                break;
            }
        }

        let mut fractional_part = 0.;
        if self.has_at_least(1)
            && self.next_byte_unchecked() == b'.'
            && self.byte_at(1).is_ascii_digit()
        {
            num_type = NumberType::Number;
            self.advance(1);
            let mut factor = 0.1;
            while let Some(digit) = self.byte_to_decimal_digit(self.next_byte_unchecked()) {
                fractional_part += digit as f64 * factor;
                factor *= 0.1;
                self.advance(1);
                if self.is_eof() {
                    break;
                }
            }
        }

        let mut value =
            (if matches!(sign, Some('-')) { -1. } else { 1. }) * (int_part + fractional_part);

        if self.has_at_least(1)
            && matches!(self.next_byte_unchecked(), b'e' | b'E')
            && (self.byte_at(1).is_ascii_digit()
                || (self.has_at_least(2)
                    && matches!(self.byte_at(1), b'+' | b'-')
                    && self.byte_at(2).is_ascii_digit()))
        {
            self.advance(1);
            let sign = match self.next_byte_unchecked() {
                b'+' => {
                    self.advance(1);
                    Some('+')
                }
                b'-' => {
                    self.advance(1);
                    Some('-')
                }
                _ => None,
            };

            let mut exponent = 0.;
            while let Some(digit) = self.byte_to_decimal_digit(self.next_byte_unchecked()) {
                exponent = exponent * 10. + digit as f64;
                self.advance(1);
                if self.is_eof() {
                    break;
                }
            }
            value *= f64::powf(
                10.,
                (if matches!(sign, Some('-')) { -1. } else { 1. }) * exponent,
            );
        }

        (value, num_type, sign)
    }

    fn has_newline_at(&self, offset: usize) -> bool {
        self.pos + offset < self.input.len()
            && matches!(self.byte_at(offset), b'\n' | b'\r' | b'\x0C')
    }

    fn is_ident_start(&self) -> bool {
        !self.is_eof()
            && match self.next_byte_unchecked() {
                b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'\0' => true,
                b'-' => {
                    self.has_at_least(1)
                        && match self.byte_at(1) {
                            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'\0' => true,
                            b'\\' => !self.has_newline_at(1),
                            b => !b.is_ascii(),
                        }
                }
                b'\\' => !self.has_newline_at(1),
                b => !b.is_ascii(),
            }
    }

    fn consume_name(&mut self) -> Cow<'a, str> {
        let start_pos = self.pos;
        let mut string_bytes;

        loop {
            if self.is_eof() {
                return Cow::Borrowed(self.slice_from(start_pos).into());
            }

            match self.next_byte_unchecked() {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' => {
                    self.advance(1);
                }
                b'\\' | b'\0' => {
                    string_bytes = self.slice_from(start_pos).as_bytes().to_owned();
                    break;
                }
                b'\x80'..=b'\xBF' | b'\xC0'..=b'\xEF' | b'\xF0'..=b'\xFF' => {
                    self.advance(1);
                }
                _ => {
                    return Cow::Borrowed(self.slice_from(start_pos).into());
                }
            }
        }

        while !self.is_eof() {
            let b = self.next_byte_unchecked();

            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' => {
                    self.advance(1);
                    string_bytes.push(b);
                }
                b'\\' => {
                    if self.has_newline_at(1) {
                        break;
                    }
                    self.advance(1);
                    let c = self.consume_escaped_code_point();
                    string_bytes.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
                }
                b'\0' => {
                    self.advance(1);
                    string_bytes.extend("\u{FFFD}".as_bytes());
                }
                b'\x80'..=b'\xBF' | b'\xC0'..=b'\xEF' | b'\xF0'..=b'\xFF' => {
                    self.advance(1);
                    string_bytes.push(b);
                }
                _ => {
                    break;
                }
            }
        }

        Cow::Owned(unsafe { String::from_utf8_unchecked(string_bytes).into() })
    }

    fn consume_hash(&mut self) -> Token<'a> {
        self.advance(1);
        if self.is_ident_start() {
            Token::IDHash(self.consume_name())
        } else if !self.is_eof() && matches!(self.next_byte_unchecked(), b'0'..=b'9' | b'-') {
            Token::Hash(self.consume_name())
        } else {
            Token::Delim('#')
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

    fn byte_to_hex_digit(&self, b: u8) -> Option<u32> {
        match b {
            b'0'..=b'9' => Some((b - b'0') as u32),
            b'a'..=b'f' => Some((b - b'a') as u32),
            b'A'..=b'F' => Some((b - b'A') as u32),
            _ => None,
        }
    }

    fn consume_hex_digits(&mut self) -> u32 {
        let mut value = 0;
        let mut digits = 0;
        while digits < 6 && !self.is_eof() {
            match self.byte_to_hex_digit(self.next_byte_unchecked()) {
                Some(digit) => {
                    value = value * 16 + digit;
                    digits += 1;
                    self.advance(1);
                }
                None => break,
            }
        }
        value
    }

    /// https://drafts.csswg.org/css-syntax/#consume-an-escaped-code-point
    fn consume_escaped_code_point(&mut self) -> char {
        if self.is_eof() {
            return '\u{FFFD}';
        }

        match self.next_byte_unchecked() {
            b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => {
                let c = self.consume_hex_digits();
                if !self.is_eof() {
                    match self.next_byte_unchecked() {
                        b' ' | b'\t' => {
                            self.advance(1);
                        }
                        b'\n' | b'\x0C' | b'\r' => {
                            self.consume_newline();
                        }
                        _ => {}
                    }
                }
                if c != 0 {
                    char::from_u32(c).unwrap_or('\u{FFFD}')
                } else {
                    '\u{FFFD}'
                }
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
        let mut string_bytes;

        loop {
            if self.is_eof() {
                self.print_parse_error("consume_string EOF");
                return Token::String(Cow::Borrowed(self.slice_from(start_pos)));
            }

            match self.next_byte_unchecked() {
                b'\'' => {
                    if single_quote {
                        let res = Token::String(Cow::Borrowed(self.slice_from(start_pos)));
                        self.advance(1);
                        return res;
                    }
                    self.advance(1);
                }
                b'"' => {
                    if !single_quote {
                        let res = Token::String(Cow::Borrowed(self.slice_from(start_pos)));
                        self.advance(1);
                        return res;
                    }
                    self.advance(1);
                }
                b'\n' | b'\r' | b'\x0C' => {
                    return Token::BadString(Cow::Borrowed(self.slice_from(start_pos)));
                }
                b'\\' | b'\0' => {
                    string_bytes = self.slice_from(start_pos).as_bytes().to_owned();
                    break;
                }
                _ => {
                    self.advance(1);
                }
            }
        }

        while !self.is_eof() {
            let b = self.next_byte_unchecked();
            match b {
                b'\n' | b'\r' | b'\x0C' => {
                    return Token::BadString(Cow::Owned(unsafe {
                        String::from_utf8_unchecked(string_bytes).into()
                    }));
                }
                b'"' => {
                    self.advance(1);
                    if !single_quote {
                        break;
                    }
                }
                b'\'' => {
                    self.advance(1);
                    if single_quote {
                        break;
                    }
                }
                b'\\' => {
                    self.advance(1);
                    if !self.is_eof() {
                        match self.next_byte_unchecked() {
                            b'\n' | b'\x0C' | b'\r' => {
                                self.consume_newline();
                            }
                            _ => {
                                let c = self.consume_escaped_code_point();
                                string_bytes.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
                            }
                        }
                    }
                    continue;
                }
                b'\0' => {
                    self.advance(1);
                    string_bytes.extend("\u{FFFD}".as_bytes());
                    continue;
                }
                _ => {
                    self.advance(1);
                }
            }

            string_bytes.push(b);
        }

        Token::String(Cow::Owned(unsafe {
            String::from_utf8_unchecked(string_bytes).into()
        }))
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
