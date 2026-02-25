use crate::parser::css::Token;

/// https://drafts.csswg.org/css-syntax/#css-tree
pub(crate) struct Stylesheet<'a> {
    pub(crate) rules: Vec<Rule<'a>>,
}

/// https://drafts.csswg.org/css-syntax/#css-rule
pub(crate) enum Rule<'a> {
    At(AtRule<'a>),
    Qualified(QualifiedRule<'a>),
}

/// https://drafts.csswg.org/css-syntax/#at-rule
pub(crate) struct AtRule<'a> {
    name: String,
    prelude: Vec<ComponentValue<'a>>,
    declarations: Vec<Declaration<'a>>,
    children: Vec<Rule<'a>>,
}

/// https://drafts.csswg.org/css-syntax/#qualified-rule
pub(crate) struct QualifiedRule<'a> {
    prelude: Vec<ComponentValue<'a>>,
    declarations: Vec<Declaration<'a>>,
    children: Vec<Rule<'a>>,
}

/// https://drafts.csswg.org/css-syntax/#declaration
pub(crate) struct Declaration<'a> {
    name: String,
    values: Vec<ComponentValue<'a>>,
    unset: bool,
}

/// https://drafts.csswg.org/css-syntax/#component-value
pub(crate) enum ComponentValue<'a> {
    PreservedToken(Token<'a>),
    Function(Function<'a>),
    SimpleBlock(SimpleBlock<'a>),
}

pub(crate) struct Function<'a> {
    name: String,
    values: Vec<ComponentValue<'a>>,
}

pub(crate) struct SimpleBlock<'a> {
    token: Token<'a>,
    values: Vec<ComponentValue<'a>>,
}
