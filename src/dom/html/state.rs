#[derive(Debug, Copy, Clone)]
pub(crate) enum AttrValueKind {
    Unquoted,
    DoubleQuoted,
    SingleQuoted,
}

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
    AttributeValue(AttrValueKind),
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    SimpleComment,
}
