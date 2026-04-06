use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;

#[derive(Clone)]
pub struct OptionContractsClient {
    pub(crate) inner: Arc<Inner>,
}

impl OptionContractsClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }
}

impl Debug for OptionContractsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.inner;
        f.debug_struct("OptionContractsClient")
            .finish_non_exhaustive()
    }
}
