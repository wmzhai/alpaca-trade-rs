use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;

#[derive(Clone)]
pub struct AssetsClient {
    pub(crate) _inner: Arc<Inner>,
}

impl AssetsClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { _inner: inner }
    }
}

impl Debug for AssetsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetsClient").finish_non_exhaustive()
    }
}
