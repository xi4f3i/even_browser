mod parser;
mod tokenizer;

pub(crate) use parser::HTMLParser;
use tokenizer::{Token, Tokenizer};

use crate::dom::{Node, NodePtr};

pub(crate) fn parse_html(input: &str) -> NodePtr {
    let doc = Node::new_doc();

    let mut parser = HTMLParser::new(input, doc);
    parser.parse();

    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::{NodePtr, NodeType};

    fn get_child(parent: NodePtr, index: usize) -> NodePtr {
        let mut curr = unsafe { parent.as_ref().first_child() };
        for i in 0..index {
            assert!(curr.is_some(), "Node has no child at index {}", i);
            curr = unsafe { curr.unwrap().as_ref().next_sibling() };
        }
        assert!(curr.is_some(), "Node has no child at index {}", index);
        curr.unwrap()
    }

    fn assert_doc(node: NodePtr) {
        match &*unsafe { node.as_ref().node_type() } {
            NodeType::Document(_) => {}
            _ => panic!("Expected Document node"),
        }
    }

    fn assert_elem(node: NodePtr, expected_tag: &str) {
        match &*unsafe { node.as_ref().node_type() } {
            NodeType::Element(e) => {
                assert_eq!(&*e.tag_name(), expected_tag, "Tag name mismatch");
            }
            _ => panic!("Expected Element node, found {:?}", node_type_name(node)),
        }
    }

    fn assert_text(node: NodePtr, expected_content: &str) {
        match &*unsafe { node.as_ref().node_type() } {
            NodeType::Text(t) => {
                assert_eq!(*t.data(), expected_content, "Text content mismatch");
            }
            _ => panic!("Expected Text node, found {:?}", node_type_name(node)),
        }
    }

    fn assert_attr(node: NodePtr, key: &str, value: &str) {
        match &*unsafe { node.as_ref().node_type() } {
            NodeType::Element(e) => {
                let attrs = e.attributes();
                let attr = attrs.iter().find(|a| &a.name == key);
                assert!(attr.is_some(), "Attribute {} not found", key);
                assert_eq!(attr.unwrap().value, value, "Attribute value mismatch");
            }
            _ => panic!("Expected Element node for attr check"),
        }
    }

    fn child_count(node: NodePtr) -> usize {
        let mut count = 0;
        let mut curr = unsafe { node.as_ref().first_child() };
        while let Some(ptr) = curr {
            count += 1;
            curr = unsafe { ptr.as_ref().next_sibling() };
        }
        count
    }

    fn node_type_name(node: NodePtr) -> &'static str {
        match &*unsafe { node.as_ref().node_type() } {
            NodeType::Document(_) => "Document",
            NodeType::Element(_) => "Element",
            NodeType::Text(_) => "Text",
        }
    }

    #[test]
    fn test_basic_structure() {
        let input = "<html><body><div></div></body></html>";
        let doc = parse_html(input);

        assert_doc(doc);

        let html = get_child(doc, 0);
        assert_elem(html, "html");

        let head = get_child(html, 0);
        assert_elem(head, "head");

        let body = get_child(html, 1);
        assert_elem(body, "body");

        // body -> div
        let div = get_child(body, 0);
        assert_elem(div, "div");
    }

    #[test]
    fn test_implicit_tags() {
        let input = "<div>Hello</div>";
        let doc = parse_html(input);

        let html = get_child(doc, 0);
        assert_elem(html, "html");

        let head = get_child(html, 0);
        assert_elem(head, "head");

        let body = get_child(html, 1);
        assert_elem(body, "body");

        let div = get_child(body, 0);
        assert_elem(div, "div");

        let text = get_child(div, 0);
        assert_text(text, "Hello");
    }

    #[test]
    fn test_attributes() {
        let input = "<div id=\"container\" class=\"main\"></div>";
        let doc = parse_html(input);

        let html = get_child(doc, 0);
        let body = get_child(html, 1);
        let div = get_child(body, 0);

        assert_elem(div, "div");
        assert_attr(div, "id", "container");
        assert_attr(div, "class", "main");
    }

    #[test]
    fn test_nested_elements() {
        let input = "<ul><li>A</li><li>B</li></ul>";
        let doc = parse_html(input);

        let body = get_child(get_child(doc, 0), 1);
        let ul = get_child(body, 0);
        assert_elem(ul, "ul");
        assert_eq!(child_count(ul), 2);

        let li1 = get_child(ul, 0);
        assert_elem(li1, "li");
        assert_text(get_child(li1, 0), "A");

        let li2 = get_child(ul, 1);
        assert_elem(li2, "li");
        assert_text(get_child(li2, 0), "B");
    }

    #[test]
    fn test_self_closing_tags() {
        let input = "<head><meta charset=\"utf-8\"></head><body>Text<br>End</body>";
        let doc = parse_html(input);

        let html = get_child(doc, 0);
        let head = get_child(html, 0);
        let body = get_child(html, 1);

        let meta = get_child(head, 0);
        assert_elem(meta, "meta");
        assert_attr(meta, "charset", "utf-8");

        assert_eq!(child_count(body), 3);

        let t1 = get_child(body, 0);
        assert_text(t1, "Text");

        let br = get_child(body, 1);
        assert_elem(br, "br");

        let t2 = get_child(body, 2);
        assert_text(t2, "End");
    }

    #[test]
    fn test_whitespace_handling() {
        let input = "
        <html>
            <body>
                <div>  content  </div>
            </body>
        </html>
        ";
        let doc = parse_html(input);

        let html = get_child(doc, 0);
        let body = get_child(html, 1);

        let mut found_div = false;
        let mut curr = unsafe { body.as_ref().first_child() };
        while let Some(ptr) = curr {
            if let NodeType::Element(e) = &*unsafe { ptr.as_ref().node_type() } {
                if &*e.tag_name() == "div" {
                    found_div = true;
                    let content = get_child(ptr, 0);
                    assert_text(content, "  content  ");
                }
            }
            curr = unsafe { ptr.as_ref().next_sibling() };
        }
        assert!(found_div, "Did not find div in body with whitespace input");
    }

    #[test]
    fn test_rcdata_elements() {
        let input = "<title><b>Bold</b></title>";
        let doc = parse_html(input);

        let html = get_child(doc, 0);
        let head = get_child(html, 0);
        let title = get_child(head, 0);

        assert_elem(title, "title");

        let text = get_child(title, 0);
        assert_text(text, "<b>Bold</b>");
    }
}
