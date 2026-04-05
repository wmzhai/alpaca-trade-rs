#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clock {
    pub timestamp: String,
    pub is_open: bool,
    pub next_open: String,
    pub next_close: String,
}
