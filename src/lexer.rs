#[derive(Debug, Clone)]
pub enum Token {
    Text(String),
    Tag(String),
}

pub fn lex(body: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut buffer = String::with_capacity(body.len());
    let mut in_tag = false;

    for c in body.chars() {
        match c {
            '<' => {
                in_tag = true;
                if !buffer.is_empty() {
                    out.push(Token::Text(buffer.clone()));
                }
                buffer.clear();
            }
            '>' => {
                in_tag = false;
                out.push(Token::Tag(buffer.clone()));
                buffer.clear();
            }
            _ => buffer.push(c),
        }
    }

    if !in_tag && !buffer.is_empty() {
        out.push(Token::Text(buffer));
    }

    out
}
