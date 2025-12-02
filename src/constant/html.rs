pub const HTML: &str = "html";
pub const SLASH_HTML: &str = "/html";
pub const HEAD: &str = "head";
pub const SLASH_HEAD: &str = "/head";
pub const BODY: &str = "body";
pub const STYLE: &str = "style";
pub const LINK: &str = "link";

pub const ATTRIBUTE_KEY_STYLE: &str = "style";
pub const ATTRIBUTE_KEY_REL: &str = "rel";
pub const ATTRIBUTE_REL_VALUE_STYLESHEET: &str = "stylesheet";
pub const ATTRIBUTE_KEY_HREF: &str = "href";

pub const SELF_CLOSING_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", LINK, "meta", "param", "source",
    "track", "wbr",
];

pub const HEAD_ELEMENTS: [&str; 9] = [
    "base", "basefont", "bgsound", "noscript", LINK, "meta", "title", STYLE, "script",
];

pub const BLOCK_ELEMENTS: [&str; 37] = [
    HTML,
    BODY,
    "article",
    "section",
    "nav",
    "aside",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hgroup",
    "header",
    "footer",
    "address",
    "p",
    "hr",
    "pre",
    "blockquote",
    "ol",
    "ul",
    "menu",
    "li",
    "dl",
    "dt",
    "dd",
    "figure",
    "figcaption",
    "main",
    "div",
    "table",
    "form",
    "fieldset",
    "legend",
    "details",
    "summary",
];
