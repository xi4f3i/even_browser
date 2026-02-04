use crate::parser::css::tokenizer::Token;

/// https://drafts.csswg.org/css-syntax/#css-tree
struct Stylesheet<'a> {
    rules: Vec<Rule<'a>>,
}

/// https://drafts.csswg.org/css-syntax/#css-rule
enum Rule<'a> {
    At(AtRule<'a>),
    Qualified(QualifiedRule<'a>),
}

/// https://drafts.csswg.org/css-syntax/#at-rule
struct AtRule<'a> {
    name: String,
    prelude: Vec<ComponentValue<'a>>,
    declarations: Vec<Declaration<'a>>,
    children: Vec<Rule<'a>>,
}

/// https://drafts.csswg.org/css-syntax/#qualified-rule
struct QualifiedRule<'a> {
    prelude: Vec<ComponentValue<'a>>,
    declarations: Vec<Declaration<'a>>,
    children: Vec<Rule<'a>>,
}

/// https://drafts.csswg.org/css-syntax/#declaration
struct Declaration<'a> {
    name: String,
    values: Vec<ComponentValue<'a>>,
    unset: bool,
}

/// https://drafts.csswg.org/css-syntax/#component-value
enum ComponentValue<'a> {
    PreservedToken(Token<'a>),
    Function(Function<'a>),
    SimpleBlock(SimpleBlock<'a>),
}

struct Function<'a> {
    name: String,
    values: Vec<ComponentValue<'a>>,
}

struct SimpleBlock<'a> {
    token: Token<'a>,
    values: Vec<ComponentValue<'a>>,
}
