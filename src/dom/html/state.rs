#[derive(Debug, Copy, Clone)]
pub(crate) enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    SelfClosingStartTag,
    SimpleComment,
}
