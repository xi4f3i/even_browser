use std::borrow::Cow;

use crate::parser::css::{Rule, Stylesheet, Token, Tokenizer};

/// https://drafts.csswg.org/css-syntax/#parsing
pub(crate) struct CSSParser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> CSSParser<'a> {
    pub(crate) fn new(input: &'a str) -> CSSParser<'a> {
        CSSParser {
            tokenizer: Tokenizer::new(input),
        }
    }

    /// https://drafts.csswg.org/css-syntax/#parse-stylesheet
    pub(crate) fn parse_stylesheet(&mut self) -> Stylesheet<'a> {
        let rules = self.consume_stylesheet_contents();
        Stylesheet { rules }
    }

    /// https://drafts.csswg.org/css-syntax/#consume-stylesheet-contents
    fn consume_stylesheet_contents(&mut self) -> Vec<Rule<'a>> {
        let mut rules = Vec::new();

        loop {
            let token = self.tokenizer.next();

            match token {
                Token::Whitespace(_) => continue, // discard a token
                Token::EOF => break,
                Token::CDO | Token::CDC => continue,
                Token::AtKeyword(name) => {
                    if let Some(rule) = self.consume_at_rule(name) {
                        rules.push(rule);
                    }
                }
                _ => {
                    if let Some(rule) = self.consume_qualified_rule(token) {
                        rules.push(rule);
                    }
                }
            }
        }

        rules
    }

    /// https://drafts.csswg.org/css-syntax/#consume-at-rule
    fn consume_at_rule(&mut self, name: Cow<'a, str>) -> Option<Rule<'a>> {

    }

    fn consume_qualified_rule() -> Option<Rule<'a>> {

    }
}
