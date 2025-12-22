#[derive(Debug)]
pub(crate) struct Doctype {
    name: String,
    public_id: Option<String>,
    system_id: Option<String>,
    force_quirks: bool,
}

#[derive(Debug)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub(crate) struct Tag {
    pub(crate) tag_name: String,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) self_closing: bool,
}

#[derive(Debug)]
pub(crate) enum Token {
    Doctype(Doctype),
    Comment(String),
    Character(char),
    StartTag(Tag),
    EndTag(String),
    EOF,
}
