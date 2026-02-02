/// https://drafts.csswg.org/css-syntax/#css-tree
struct Stylesheet {
    rules: Vec<Rule>,
}

/// https://drafts.csswg.org/css-syntax/#css-rule
enum Rule {
    At(AtRule),
    Qualified(QualifiedRule),
}

/// https://drafts.csswg.org/css-syntax/#at-rule
struct AtRule {}

/// https://drafts.csswg.org/css-syntax/#qualified-rule
struct QualifiedRule {}
