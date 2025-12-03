use crate::constant::common::{
    CLOSING_BRACE, COLON, DASH, DOT, HASH, OPENING_BRACE, PERCENT, SEMICOLON, SLASH,
};
use crate::parser::selector::Selector;
use std::collections::HashMap;

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

    fn whitespace(&mut self) -> bool {
        let mut flag = false;

        while self.idx < self.chars.len() && self.chars[self.idx].is_whitespace() {
            self.idx += 1;
            flag = true;
        }

        flag
    }

    fn comment(&mut self) -> bool {
        let mut flag = false;

        if self.idx + 1 >= self.chars.len() {
            return flag;
        }

        if self.chars[self.idx] == SLASH && self.chars[self.idx + 1] == '*' {
            while self.idx + 1 < self.chars.len() {
                if self.chars[self.idx] == '*' && self.chars[self.idx + 1] == SLASH {
                    self.idx += 2;
                    flag = true;
                    break;
                }
                self.idx += 1;
            }
        }

        flag
    }

    fn comment_and_whitespace(&mut self) {
        while self.idx < self.chars.len() {
            let f1 = self.whitespace();
            let f2 = self.comment();
            if !f1 && !f2 {
                break;
            }
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
        self.comment_and_whitespace();
        self.literal(COLON)?;
        self.comment_and_whitespace();
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
        self.comment_and_whitespace();
        self.literal(SEMICOLON)?;
        self.comment_and_whitespace();
        Ok(())
    }

    pub fn body(&mut self) -> Result<CSSRuleBody, CSSParserError> {
        let mut pairs = HashMap::new();

        while self.idx < self.chars.len() && self.chars[self.idx] != CLOSING_BRACE {
            match self.pair_sequence(&mut pairs) {
                Ok(()) => {}
                Err(msg) => {
                    println!("{}", msg);

                    if let Some(why) = self.ignore_until(&[SEMICOLON, CLOSING_BRACE])
                        && why == SEMICOLON
                    {
                        self.literal(SEMICOLON)?;
                        self.comment_and_whitespace();
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(pairs)
    }

    fn selector(&mut self) -> Result<Selector, CSSParserError> {
        let mut out = Selector::new_tag(self.word()?.to_lowercase());

        self.comment_and_whitespace();

        while self.idx < self.chars.len() && self.chars[self.idx] != OPENING_BRACE {
            let tag = self.word()?;
            let descendant = Selector::new_tag(tag.to_lowercase());
            out = Selector::new_descendant(out, descendant);
            self.comment_and_whitespace();
        }

        Ok(out)
    }

    fn parse_sequence(&mut self) -> Result<CSSRule, CSSParserError> {
        self.comment_and_whitespace();
        let selector = self.selector()?;
        self.literal(OPENING_BRACE)?;
        self.comment_and_whitespace();
        let body = self.body()?;
        self.literal(CLOSING_BRACE)?;
        Ok((selector, body))
    }

    pub fn parse(&mut self) -> Result<CSSRules, CSSParserError> {
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
                        self.literal(CLOSING_BRACE)?;
                        self.comment_and_whitespace();
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(rules)
    }
}
