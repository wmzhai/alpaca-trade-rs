pub trait Observer: Send + Sync + 'static {
    fn on_request_start(&self, _event: &RequestStart) {}
    fn on_retry(&self, _event: &RetryEvent) {}
    fn on_response(&self, _event: &ResponseEvent) {}
    fn on_error(&self, _event: &ErrorEvent) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestStart {
    pub endpoint: String,
    pub method: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryEvent {
    pub endpoint: String,
    pub method: String,
    pub attempt: usize,
    pub status: Option<u16>,
    pub wait_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseEvent {
    pub endpoint: String,
    pub method: String,
    pub status: u16,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorEvent {
    pub endpoint: String,
    pub method: String,
    pub status: Option<u16>,
    pub request_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct NoopObserver;

impl Observer for NoopObserver {}
