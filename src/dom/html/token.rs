
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attribute {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagKind {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tag {
    pub(crate) name: String,
    pub(crate) self_closing: bool,
    pub(crate) attributes: Vec<Attribute>,
}

impl Tag {
    pub(crate) fn new(name: String, self_closing: bool, attributes: Vec<Attribute>) -> Tag {
        Tag {
            name,
            self_closing,
            attributes,
        }
    }
}

/// https://html.spec.whatwg.org/multipage/parsing.html#tokenization
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    StartTag(Tag),
    EndTag(Tag),
    Character(char),
    EOF,
}
