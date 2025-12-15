use crate::browser::Browser;
use net::url::URL;

mod browser;
mod constant;
mod layout;
mod net;
mod parser;
mod dom;

fn main() {
    let url_str = "https://browser.engineering/styles.html";
    let url = URL::new(url_str);
    let mut browser = Browser::new();
    browser.load(&url);
    browser.run();
}
