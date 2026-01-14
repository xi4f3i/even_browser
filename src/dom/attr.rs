use crate::dom::Atom;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Attr {
    pub(crate) name: Atom,
    pub(crate) value: String,
}
