pub fn lex(body: &str) -> String {
    let mut text = String::with_capacity(body.len());
    let mut in_tag = false;

    for c in body.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }

    text
}
