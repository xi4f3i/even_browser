use crate::{
    dom::{Node, NodePtr, NodeType},
    parser::html::{Token, Tokenizer, tokenizer::Tag},
};

const IMPLICIT_TAGS: [&str; 4] = ["head", "body", "html", "br"];
const SELF_CLOSING_HEAD_TAGS: [&str; 4] = ["base", "basefont", "bgsound", "link"];
const VOID_TAGS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

#[derive(Clone, Copy, Debug)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    AfterBody,
    AfterAfterBody,
    Text,
}

enum ProcessResult {
    Stop,
    Ignore,
    Switch(InsertionMode),
    Reprocess(InsertionMode, Token),
    Continue,
}

pub(crate) struct HTMLParser<'a> {
    tokenizer: Tokenizer<'a>,
    mode: InsertionMode,
    orig_mode: InsertionMode,
    doc: NodePtr,
    head: Option<NodePtr>,
    open_elems: Vec<NodePtr>,
}

impl<'a> HTMLParser<'a> {
    pub(crate) fn new(input: &'a str, doc: NodePtr) -> HTMLParser<'a> {
        HTMLParser {
            tokenizer: Tokenizer::new(input),
            mode: InsertionMode::Initial,
            orig_mode: InsertionMode::Initial,
            doc,
            head: None,
            open_elems: vec![doc],
        }
    }

    pub(crate) fn parse(&mut self) {
        let mut token = None;

        loop {
            let next_token = token.take().unwrap_or_else(|| self.tokenizer.next());
            let res = self.process(next_token);

            match res {
                ProcessResult::Stop => return,
                ProcessResult::Ignore | ProcessResult::Continue => {}
                ProcessResult::Switch(mode) => {
                    self.mode = mode;
                }
                ProcessResult::Reprocess(mode, t) => {
                    self.mode = mode;
                    token = Some(t);
                }
            }
        }
    }

    fn process(&mut self, token: Token) -> ProcessResult {
        match self.mode {
            InsertionMode::Initial => self.handle_initial(token),
            InsertionMode::BeforeHtml => self.handle_before_html(token),
            InsertionMode::BeforeHead => self.handle_before_head(token),
            InsertionMode::InHead => self.handle_in_head(token),
            InsertionMode::AfterHead => self.handle_after_head(token),
            InsertionMode::InBody => self.handle_in_body(token),
            InsertionMode::AfterBody => self.handle_after_body(token),
            InsertionMode::AfterAfterBody => self.handle_after_after_body(token),
            InsertionMode::Text => self.handle_text(token),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
    fn handle_after_after_body(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) if self.is_whitespace(ch) => self.handle_in_body(token),
            Token::StartTag(tag) if tag.name == "html" => self.handle_in_body(Token::StartTag(tag)),
            Token::EOF => ProcessResult::Stop,
            _ => {
                self.print_parse_error("handle_after_after_body unexpected token");
                ProcessResult::Reprocess(InsertionMode::InBody, token)
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
    fn handle_after_body(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) if self.is_whitespace(ch) => self.handle_in_body(token),
            Token::StartTag(tag) if tag.name == "html" => self.handle_in_body(Token::StartTag(tag)),
            Token::EndTag(tag) if tag.name == "html" => {
                ProcessResult::Switch(InsertionMode::AfterAfterBody)
            }
            Token::EOF => ProcessResult::Stop,
            _ => {
                self.print_parse_error("handle_after_body unexpected token");
                ProcessResult::Reprocess(InsertionMode::InBody, token)
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incdata
    fn handle_text(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) => {
                self.insert_char(ch);
                ProcessResult::Continue
            }
            Token::EOF => {
                self.print_parse_error("handle_text unexpected EOF");
                self.open_elems.pop();
                ProcessResult::Reprocess(self.orig_mode, token)
            }
            Token::EndTag(_) => {
                self.open_elems.pop();
                ProcessResult::Switch(self.orig_mode)
            }
            _ => ProcessResult::Switch(self.orig_mode),
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody
    fn handle_in_body(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) => {
                // https://html.spec.whatwg.org/multipage/parsing.html#reconstruct-the-active-formatting-elements
                self.insert_char(ch);
                ProcessResult::Continue
            }
            Token::StartTag(tag) if tag.name == "html" => {
                self.print_parse_error("handle_in_body unexpected start tag: html");
                ProcessResult::Ignore
            }
            Token::StartTag(tag)
                if tag.name == "base"
                    || tag.name == "basefont"
                    || tag.name == "bgsound"
                    || tag.name == "link"
                    || tag.name == "meta"
                    || tag.name == "noframes"
                    || tag.name == "script"
                    || tag.name == "style"
                    || tag.name == "template"
                    || tag.name == "title" =>
            {
                self.handle_in_head(Token::StartTag(tag))
            }
            Token::StartTag(tag) if tag.name == "body" => {
                self.print_parse_error("handle_in_body unexpected start tag: body");
                ProcessResult::Ignore
            }
            Token::EOF => ProcessResult::Stop,
            Token::EndTag(tag) if tag.name == "body" => {
                ProcessResult::Switch(InsertionMode::AfterBody)
            }
            Token::EndTag(tag) if tag.name == "html" => {
                ProcessResult::Reprocess(InsertionMode::AfterBody, Token::EndTag(tag))
            }
            Token::StartTag(tag) => {
                // https://html.spec.whatwg.org/multipage/parsing.html#reconstruct-the-active-formatting-elements
                self.insert_html_elem(tag);
                ProcessResult::Continue
            }
            Token::EndTag(tag) => {
                let node = unsafe { self.cur_node().as_ref() };
                if let NodeType::Element(elem) = &*node.node_type()
                    && *elem.tag_name() == tag.name
                {
                    self.open_elems.pop();
                }
                ProcessResult::Continue
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-after-head-insertion-mode
    fn handle_after_head(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) if self.is_whitespace(ch) => {
                self.insert_char(ch);
                ProcessResult::Continue
            }
            Token::StartTag(tag) if tag.name == "html" => self.handle_in_body(Token::StartTag(tag)),
            Token::StartTag(tag) if tag.name == "body" => {
                self.insert_html_elem(tag);
                ProcessResult::Switch(InsertionMode::InBody)
            }
            Token::EndTag(tag) if tag.name == "body" || tag.name == "html" || tag.name == "br" => {
                self.insert_html_elem(Tag {
                    name: String::from("body"),
                    self_closing: false,
                    attrs: Vec::new(),
                });
                ProcessResult::Reprocess(InsertionMode::InBody, Token::EndTag(tag))
            }
            Token::StartTag(tag) if tag.name == "head" => {
                self.print_parse_error("handle_after_head unexpected start tag: head");
                ProcessResult::Ignore
            }
            Token::EndTag(tag) => {
                self.print_parse_error(&format!(
                    "handle_after_head unexpected end tag: {}",
                    tag.name
                ));
                ProcessResult::Ignore
            }
            _ => {
                self.insert_html_elem(Tag {
                    name: String::from("body"),
                    self_closing: false,
                    attrs: Vec::new(),
                });
                ProcessResult::Reprocess(InsertionMode::InBody, token)
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
    fn handle_in_head(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) if self.is_whitespace(ch) => {
                self.insert_char(ch);
                ProcessResult::Continue
            }
            Token::StartTag(tag) if tag.name == "html" => self.handle_in_body(Token::StartTag(tag)),
            Token::StartTag(mut tag)
                if tag.name == "meta" || SELF_CLOSING_HEAD_TAGS.contains(&tag.name.as_str()) =>
            {
                tag.self_closing = true;
                self.insert_html_elem(tag);
                ProcessResult::Continue
            }
            Token::StartTag(tag)
                if tag.name == "title"
                    || tag.name == "noscript"
                    || tag.name == "noframes"
                    || tag.name == "style" =>
            {
                // https://html.spec.whatwg.org/multipage/parsing.html#generic-rcdata-element-parsing-algorithm
                // https://html.spec.whatwg.org/multipage/parsing.html#generic-raw-text-element-parsing-algorithm
                self.insert_html_elem(tag);
                self.orig_mode = InsertionMode::InHead;
                ProcessResult::Switch(InsertionMode::Text)
            }
            Token::EndTag(tag) if tag.name == "head" => {
                self.open_elems.pop();
                ProcessResult::Switch(InsertionMode::AfterHead)
            }
            Token::EndTag(tag) if tag.name == "body" || tag.name == "html" || tag.name == "br" => {
                self.open_elems.pop();
                ProcessResult::Reprocess(InsertionMode::AfterHead, Token::EndTag(tag))
            }
            Token::StartTag(tag) if tag.name == "head" => {
                self.print_parse_error("handle_in_head unexpected start tag: head");
                ProcessResult::Ignore
            }
            Token::EndTag(tag) => {
                self.print_parse_error(&format!("handle_in_head unexpected end tag: {}", tag.name));
                ProcessResult::Ignore
            }
            _ => {
                self.open_elems.pop();
                ProcessResult::Reprocess(InsertionMode::AfterHead, token)
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-a-character
    fn insert_char(&mut self, ch: char) {
        let parent_ptr = self.cur_node();
        let parent = unsafe { parent_ptr.as_ref() };

        if matches!(*parent.node_type(), NodeType::Document(_)) {
            return;
        }

        let child_ptr = parent.last_child();
        if let Some(child) = child_ptr
            && let child = unsafe { child.as_ref() }
            && let NodeType::Text(t) = &*child.node_type()
        {
            t.append_data(ch);
        } else {
            let text = Node::new_text(Some(parent_ptr), child_ptr, ch);
            parent.append_child(text);
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
    fn handle_before_head(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) if self.is_whitespace(ch) => ProcessResult::Ignore,
            Token::StartTag(tag) if tag.name == "html" => self.handle_in_body(Token::StartTag(tag)),
            Token::StartTag(tag) if tag.name == "head" => {
                let head = self.insert_html_elem(tag);
                self.head = Some(head);
                ProcessResult::Switch(InsertionMode::InHead)
            }
            Token::EndTag(tag) if IMPLICIT_TAGS.contains(&tag.name.as_str()) => {
                let head = self.insert_html_elem(Tag {
                    name: String::from("head"),
                    self_closing: false,
                    attrs: Vec::new(),
                });
                self.head = Some(head);
                ProcessResult::Reprocess(InsertionMode::InHead, Token::EndTag(tag))
            }
            Token::EndTag(tag) => {
                self.print_parse_error(&format!(
                    "handle_before_head unexpected end tag: {}",
                    tag.name
                ));
                ProcessResult::Ignore
            }
            _ => {
                let head = self.insert_html_elem(Tag {
                    name: String::from("head"),
                    self_closing: false,
                    attrs: Vec::new(),
                });
                self.head = Some(head);
                ProcessResult::Reprocess(InsertionMode::InHead, token)
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#insert-an-html-element
    fn insert_html_elem(&mut self, tag: Tag) -> NodePtr {
        self.create_elem_for_token(tag)
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
    fn handle_before_html(&mut self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) if self.is_whitespace(ch) => ProcessResult::Ignore,
            Token::StartTag(tag) if tag.name == "html" => {
                self.create_elem_for_token(tag);
                ProcessResult::Switch(InsertionMode::BeforeHead)
            }
            Token::EndTag(tag) if IMPLICIT_TAGS.contains(&tag.name.as_str()) => {
                self.create_elem_for_token(Tag {
                    name: String::from("html"),
                    self_closing: false,
                    attrs: Vec::new(),
                });
                ProcessResult::Reprocess(InsertionMode::BeforeHead, Token::EndTag(tag))
            }
            Token::EndTag(tag) => {
                self.print_parse_error(&format!(
                    "handle_before_html unexpected end tag: {}",
                    tag.name
                ));
                ProcessResult::Ignore
            }
            _ => {
                self.create_elem_for_token(Tag {
                    name: String::from("html"),
                    self_closing: false,
                    attrs: Vec::new(),
                });
                ProcessResult::Reprocess(InsertionMode::BeforeHead, token)
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#create-an-element-for-the-token
    fn create_elem_for_token(&mut self, tag: Tag) -> NodePtr {
        let parent_ptr = self.cur_node();
        let parent = unsafe { parent_ptr.as_ref() };

        let prev_sibling_ptr = parent.last_child();

        let Tag {
            name,
            self_closing,
            attrs,
        } = tag;

        let is_void_tag = VOID_TAGS.contains(&name.as_str());

        let elem = Node::new_elem(
            Some(parent_ptr),
            prev_sibling_ptr,
            name,
            self_closing,
            attrs,
        );

        parent.append_child(elem);

        if !self_closing && !is_void_tag {
            self.open_elems.push(elem);
        }

        elem
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#current-node
    fn cur_node(&self) -> NodePtr {
        self.open_elems
            .last()
            .copied()
            .expect("open_elems is empty")
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
    fn handle_initial(&self, token: Token) -> ProcessResult {
        match token {
            Token::Char(ch) if self.is_whitespace(ch) => ProcessResult::Ignore,
            _ => ProcessResult::Reprocess(InsertionMode::BeforeHtml, token),
        }
    }

    fn is_whitespace(&self, ch: char) -> bool {
        ch == '\t' || ch == '\n' || ch == '\x0C' || ch == '\r' || ch == ' '
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parse-errors
    fn print_parse_error(&self, msg: &str) {
        println!("[HTMLParser] Parse error: {}", msg);
    }
}
