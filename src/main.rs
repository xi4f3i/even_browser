use crate::browser::Browser;
use net::url::URL;

mod browser;
mod constant;
mod layout;
mod net;
mod parser;
mod dom;
mod html_parser_backup;
mod html_parser;

fn main() {
    let url_str = "https://browser.engineering/styles.html";
    let url = URL::new(url_str);
    let mut browser = Browser::new();
    browser.load(&url);
    browser.run();
}
