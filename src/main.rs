use crate::{browser::Browser, url::URL};

mod browser;
mod url;

fn main() {
    let url_str = "https://browser.engineering/examples/xiyouji.html";
    let url = URL::new(url_str);
    let body = url.request();
    show(&body);

    let mut browser = Browser::default();
    browser.init();
}

fn show(body: &str) {
    let mut in_tag = false;
    for c in body.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            print!("{}", c);
        }
    }
}
