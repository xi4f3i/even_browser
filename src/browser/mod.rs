mod browser;

use browser::Browser;

use crate::net::URL;

pub fn run(url: &str) {
    let url = URL::new(url);
    let mut browser = Browser::new();
    browser.load(&url);
    browser.run();
}
