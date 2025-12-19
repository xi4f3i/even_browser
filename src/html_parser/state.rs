#[derive(Debug)]
pub(crate) enum State {
    Data,
    TagOpen,
    TagName,
    BeforeAttributeName,
    SelfClosingStartTag,
    EndTagOpen,
    MarkupDeclarationOpen,
    BogusComment,
}
