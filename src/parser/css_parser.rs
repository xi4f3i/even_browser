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
pub type CSSRuleBody = HashMap<String, String>;
pub type CSSRule = (Selector, CSSRuleBody);
pub type CSSRules = Vec<CSSRule>;

#[derive(Debug)]
pub struct CSSParser {
    chars: Vec<char>,
    idx: usize,
}

impl CSSParser {
    pub fn new(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        Self { chars, idx: 0 }
    }

    fn whitespace(&mut self) {
        while self.idx < self.chars.len() && self.chars[self.idx].is_whitespace() {
            self.idx += 1;
        }
    }

    fn literal(&mut self, literal: char) -> Result<(), CSSParserError> {
        if self.idx >= self.chars.len() || self.chars[self.idx] != literal {
            return Err(format!(
                "Error: literal idx={} char={}",
                self.idx,
                self.chars.get(self.idx).unwrap_or(&' ')
            ));
        }

        self.idx += 1;

        Ok(())
    }

    fn word(&mut self) -> Result<String, CSSParserError> {
        let start = self.idx;

        while self.idx < self.chars.len() {
            if self.chars[self.idx].is_alphanumeric()
                || self.chars[self.idx] == HASH
                || self.chars[self.idx] == DASH
                || self.chars[self.idx] == DOT
                || self.chars[self.idx] == PERCENT
            {
                self.idx += 1;
            } else {
                break;
            }
        }

        if self.idx == start {
            return Err(format!(
                "Error: word idx={} chars={}",
                self.idx,
                self.chars[start..self.idx].iter().collect::<String>()
            ));
        }

        Ok(self.chars[start..self.idx].iter().collect())
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
        while self.idx < self.chars.len() {
            let c = self.chars[self.idx];
            if chars.contains(&c) {
                return Some(c);
            } else {
                self.idx += 1;
            }
        }

        None
    }

    fn pair_sequence(&mut self, pairs: &mut CSSRuleBody) -> Result<(), CSSParserError> {
        let (property, value) = self.pair()?;
        pairs.insert(property, value);
        self.whitespace();
        self.literal(SEMICOLON)?;
        self.whitespace();
        Ok(())
    }

    pub fn body(&mut self) -> CSSRuleBody {
        let mut pairs = HashMap::new();

        while self.idx < self.chars.len() && self.chars[self.idx] != CLOSING_BRACE {
            match self.pair_sequence(&mut pairs) {
                Ok(()) => {}
                Err(msg) => {
                    println!("{}", msg);

                    if let Some(why) = self.ignore_until(&IGNORE_UNTIL_CHARS)
                        && why == SEMICOLON
                    {
                        match self.literal(SEMICOLON) {
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

        while self.idx < self.chars.len() && self.chars[self.idx] != OPENING_BRACE {
            let tag = self.word()?;
            let descendant = Selector::new_tag(tag.to_lowercase());
            out = Selector::new_descendant(out, descendant);
            self.whitespace();
        }

        Ok(out)
    }

    fn parse_sequence(&mut self) -> Result<CSSRule, CSSParserError> {
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

        while self.idx < self.chars.len() {
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
