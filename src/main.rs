use crate::browser::Browser;
use net::url::Url;

mod browser;
mod constant;
mod dom;
mod html_parser;
mod layout;
mod net;
mod parser;

fn main() {
    let url_str = "https://browser.engineering/styles.html";
    let url = Url::new(url_str);
    let mut browser = Browser::new();
    browser.load(&url);
    browser.run();
}
