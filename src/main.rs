use net::url::URL;
use crate::browser::Browser;

mod browser;
mod constant;
mod layout;
mod parser;
mod net;

fn main() {
    let url_str = "https://browser.engineering/examples/xiyouji.html";
    let url = URL::new(url_str);
    let mut browser = Browser::new();
    browser.load(&url);
    browser.run();
}
