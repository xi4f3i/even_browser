pub(crate) struct Attribute {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TagKind {
    Start,
    End,
}

pub(crate) struct Tag {
    name: String,
    self_closing: bool,
    attributes: Vec<Attribute>,
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
pub(crate) enum Token {
    StartTag(Tag),
    EndTag(Tag),
    Character(char),
    EOF,
}
