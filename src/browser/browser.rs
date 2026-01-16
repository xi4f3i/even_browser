use crate::{net::URL, parser::parse_html};

macro_rules! time {
    ($name:expr, $expr:expr) => {{
        let start = std::time::Instant::now();
        let result = $expr;
        println!("{} cost: {:?}", $name, start.elapsed());
        result
    }};
}

pub(crate) struct Browser {}

impl Browser {
    pub(crate) fn new() -> Browser {
        Browser {}
    }

    pub(crate) fn load(&mut self, url: &URL) {
        let body = time!("HTTP Request", url.request());
        let doc = time!("Parse HTML", parse_html(&body));

        // #[cfg(debug_assertions)]
        // unsafe {
        //     doc.as_ref().print(0)
        // };
    }

    pub(crate) fn run(&mut self) {}
}
