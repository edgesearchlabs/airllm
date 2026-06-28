#[derive(Default, Clone)]
pub struct App {
    pub output: String,
    pub status: String,
}

impl App {
    pub fn push(&mut self, chunk: &str) {
        self.output.push_str(chunk);
        self.output.push(' ');
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }
}
