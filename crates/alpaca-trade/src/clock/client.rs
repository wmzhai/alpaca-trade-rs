use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;

#[derive(Clone)]
pub struct ClockClient {
    _inner: Arc<Inner>,
}

impl ClockClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { _inner: inner }
    }
}

impl Debug for ClockClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClockClient").finish_non_exhaustive()
    }
}
