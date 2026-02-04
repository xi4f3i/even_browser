use crate::parser::css::tokenizer::Tokenizer;

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
}
