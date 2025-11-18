use crate::{browser::Browser, url::URL};

mod browser;
mod constant;
mod layout;
mod lexer;
mod url;

fn main() {
    let url_str = "https://browser.engineering/examples/xiyouji.html";
    let url = URL::new(url_str);
    let mut browser = Browser::default();
    browser.load(&url);
    browser.init();
}
