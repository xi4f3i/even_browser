#[derive(Debug, Copy, Clone)]
pub(crate) enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InBody,
}
