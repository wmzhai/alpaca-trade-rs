#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_get_attempts: usize,
    pub base_delay_ms: u64,
}

impl RetryPolicy {
    pub fn trading_safe() -> Self {
        Self {
            max_get_attempts: 2,
            base_delay_ms: 200,
        }
    }

    pub(crate) fn should_retry(
        &self,
        method: &reqwest::Method,
        status: Option<reqwest::StatusCode>,
        attempt: usize,
    ) -> bool {
        method == reqwest::Method::GET
            && attempt < self.max_get_attempts
            && status.is_some_and(|status| {
                status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
            })
    }

    pub(crate) fn wait_ms(&self, attempt: usize) -> u64 {
        self.base_delay_ms.saturating_mul(attempt as u64)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::trading_safe()
    }
}

#[cfg(test)]
mod tests {
    use reqwest::{Method, StatusCode};

    use super::RetryPolicy;

    #[test]
    fn trading_safe_policy_only_retries_get_requests() {
        let policy = RetryPolicy::trading_safe();

        assert!(policy.should_retry(&Method::GET, Some(StatusCode::TOO_MANY_REQUESTS), 1));
        assert!(!policy.should_retry(&Method::POST, Some(StatusCode::TOO_MANY_REQUESTS), 1));
        assert!(!policy.should_retry(&Method::DELETE, Some(StatusCode::INTERNAL_SERVER_ERROR), 1,));
    }
}
