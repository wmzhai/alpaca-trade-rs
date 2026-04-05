/// Trading-safe retry configuration for idempotent HTTP requests.
///
/// The current policy only retries `GET` requests for `429` and `5xx` responses.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum total number of `GET` attempts, including the first request.
    ///
    /// For example:
    /// - `1` disables retry and sends the request only once
    /// - `2` allows one retry after the first failed `GET`
    pub max_get_attempts: usize,
    /// Base delay in milliseconds multiplied by the current attempt number.
    pub base_delay_ms: u64,
}

impl RetryPolicy {
    /// Returns the default Trading-safe policy.
    ///
    /// This policy allows up to two total `GET` attempts, which means one retry
    /// after the initial failed `GET`.
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

    #[test]
    fn max_get_attempts_counts_total_attempts() {
        let policy = RetryPolicy {
            max_get_attempts: 1,
            base_delay_ms: 0,
        };

        assert!(
            !policy.should_retry(&Method::GET, Some(StatusCode::TOO_MANY_REQUESTS), 1),
            "one total attempt disables retry"
        );
    }
}
