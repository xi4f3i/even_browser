pub const WIDTH: f32 = 800.0;
pub const HEIGHT: f32 = 600.0;
pub const HSTEP: f32 = 20.0;
pub const VSTEP: f32 = 18.0;
pub const SCROLL_STEP: f32 = 100.0;

pub const SELF_CLOSING_TAGS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

pub const HEAD_TAGS: [&str; 9] = [
    "base", "basefont", "bgsound", "noscript", "link", "meta", "title", "style", "script",
];

pub const BLOCK_ELEMENTS: [&str; 37] = [
    "html", "body", "article", "section", "nav", "aside",
    "h1", "h2", "h3", "h4", "h5", "h6", "hgroup", "header",
    "footer", "address", "p", "hr", "pre", "blockquote",
    "ol", "ul", "menu", "li", "dl", "dt", "dd", "figure",
    "figcaption", "main", "div", "table", "form", "fieldset",
    "legend", "details", "summary"
];