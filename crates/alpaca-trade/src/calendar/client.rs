use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;

#[derive(Clone)]
pub struct CalendarClient {
    _inner: Arc<Inner>,
}

impl CalendarClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { _inner: inner }
    }
}

impl Debug for CalendarClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarClient").finish_non_exhaustive()
    }
}
