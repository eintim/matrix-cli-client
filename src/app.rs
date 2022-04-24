use url::Url;

pub struct App {
    /// History of recorded messages
    pub messages: Vec<(String, String)>,
    pub logged_in: bool,
    pub home_server: Url,
}

impl Default for App {
    fn default() -> App {
        App {
            messages: Vec::new(),
            logged_in: false,
            home_server: Url::parse("https://matrix.org").unwrap(),
        }
    }
}
