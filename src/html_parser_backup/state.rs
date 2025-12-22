#[derive(Debug)]
pub(crate) enum State {
    Data,
    TagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    EndTagOpen,
    MarkupDeclarationOpen,
    BogusComment,
    // CharacterReference,
}
