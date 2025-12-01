use crate::parser::selector::Selector;
use std::collections::HashMap;

const COLON: char = ':';
const HASH: char = '#';
const DASH: char = '-';
const DOT: char = '.';
pub const PERCENT: char = '%';
const SEMICOLON: char = ';';
const OPENING_BRACE: char = '{';
const CLOSING_BRACE: char = '}';
const IGNORE_UNTIL_CHARS: [char; 2] = [SEMICOLON, CLOSING_BRACE];

pub type CSSParserError = String;
pub type CSSRule = (Selector, HashMap<String, String>); // (selector, body)
pub type CSSRules = Vec<CSSRule>;

#[derive(Debug)]
pub struct CSSParser {
    chars: Vec<char>,
    index: usize,
    len: usize,
}

impl CSSParser {
    pub fn new(s: &str) -> Self {
        let chars = s.chars().collect();
        let len = s.len();
        Self {
            chars,
            index: 0,
            len,
        }
    }

    fn whitespace(&mut self) {
        while self.index < self.len && self.chars[self.index].is_whitespace() {
            self.index += 1;
        }
    }

    fn literal(&mut self, literal: char) -> Result<(), CSSParserError> {
        if self.index >= self.len || self.chars[self.index] != literal {
            return Err("Error: literal".to_string());
        }

        self.index += 1;

        Ok(())
    }

    fn word(&mut self) -> Result<String, CSSParserError> {
        let start = self.index;

        while self.index < self.len {
            if self.chars[self.index].is_alphanumeric()
                || self.chars[self.index] == HASH
                || self.chars[self.index] == DASH
                || self.chars[self.index] == DOT
                || self.chars[self.index] == PERCENT
            {
                self.index += 1;
            } else {
                break;
            }
        }

        if self.index == start {
            return Err("Error: word".to_string());
        }

        Ok(self.chars[start..self.index].iter().collect())
    }

    // (property, value)
    fn pair(&mut self) -> Result<(String, String), CSSParserError> {
        let property = self.word()?;
        self.whitespace();
        self.literal(COLON)?;
        self.whitespace();
        let value = self.word()?;
        Ok((property.to_lowercase(), value))
    }

    fn ignore_until(&mut self, chars: &[char]) -> Option<char> {
        while self.index < self.len {
            let c = self.chars[self.index];
            if chars.contains(&c) {
                return Some(c);
            } else {
                self.index += 1;
            }
        }

        None
    }

    fn pair_sequence(&mut self, pairs: &mut HashMap<String, String>) -> Result<(), CSSParserError> {
        let (property, value) = self.pair()?;
        pairs.insert(property, value);
        self.whitespace();
        self.literal(SEMICOLON)?;
        self.whitespace();
        Ok(())
    }

    pub fn body(&mut self) -> HashMap<String, String> {
        let mut pairs = HashMap::new();

        while self.index < self.len && self.chars[self.index] != CLOSING_BRACE {
            match self.pair_sequence(&mut pairs) {
                Ok(()) => {}
                Err(msg) => {
                    println!("{}", msg);

                    if let Some(why) = self.ignore_until(&IGNORE_UNTIL_CHARS)
                        && why == CLOSING_BRACE
                    {
                        match self.literal(CLOSING_BRACE) {
                            Ok(_) => {}
                            Err(msg) => {
                                println!("{}", msg);
                            }
                        };
                        self.whitespace();
                    } else {
                        break;
                    }
                }
            }
        }

        pairs
    }

    fn selector(&mut self) -> Result<Selector, CSSParserError> {
        let mut out = Selector::new_tag(self.word()?.to_lowercase());

        self.whitespace();

        while self.index < self.len && self.chars[self.index] != OPENING_BRACE {
            let descendant = Selector::new_tag(self.word()?.to_lowercase());
            out = Selector::new_descendant(out, descendant);
            self.whitespace();
        }

        Ok(out)
    }

    fn parse_sequence(&mut self) -> Result<(Selector, HashMap<String, String>), CSSParserError> {
        self.whitespace();
        let selector = self.selector()?;
        self.literal(OPENING_BRACE)?;
        self.whitespace();
        let body = self.body();
        self.literal(CLOSING_BRACE)?;
        Ok((selector, body))
    }

    pub fn parse(&mut self) -> CSSRules {
        let mut rules = vec![];

        while self.index < self.len {
            match self.parse_sequence() {
                Ok((selector, body)) => {
                    rules.push((selector, body));
                }
                Err(msg) => {
                    println!("{}", msg);
                    if let Some(why) = self.ignore_until(&[CLOSING_BRACE])
                        && why == CLOSING_BRACE
                    {
                        match self.literal(CLOSING_BRACE) {
                            Ok(_) => {}
                            Err(msg) => {
                                println!("{}", msg);
                            }
                        };
                        self.whitespace();
                    } else {
                        break;
                    }
                }
            }
        }

        rules
    }
}
