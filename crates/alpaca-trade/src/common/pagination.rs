use std::{collections::HashSet, future::Future};

use crate::error::Error;

#[allow(dead_code)]
pub(crate) trait PaginatedRequest: Clone {
    fn with_page_token(&self, page_token: Option<String>) -> Self;
}

#[allow(dead_code)]
pub(crate) trait PaginatedResponse: Sized {
    fn next_page_token(&self) -> Option<&str>;
    fn merge_page(&mut self, next: Self) -> Result<(), Error>;
    fn clear_next_page_token(&mut self);
}

fn pagination_contract_violation(message: impl Into<String>) -> Error {
    Error::InvalidRequest(format!(
        "pagination contract violation: {}",
        message.into()
    ))
}

#[allow(dead_code)]
pub(crate) async fn collect_all<Request, Response, Fetch, FutureOutput>(
    initial_request: Request,
    mut fetch_page: Fetch,
) -> Result<Response, Error>
where
    Request: PaginatedRequest,
    Response: PaginatedResponse,
    Fetch: FnMut(Request) -> FutureOutput,
    FutureOutput: Future<Output = Result<Response, Error>>,
{
    let mut combined = fetch_page(initial_request.clone()).await?;
    let mut seen_page_tokens = HashSet::new();

    while let Some(page_token) = combined.next_page_token().map(str::to_owned) {
        if !seen_page_tokens.insert(page_token.clone()) {
            return Err(pagination_contract_violation(format!(
                "repeated next_page_token `{page_token}`"
            )));
        }

        let next_page = fetch_page(initial_request.with_page_token(Some(page_token))).await?;
        combined.merge_page(next_page)?;
    }

    combined.clear_next_page_token();
    Ok(combined)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::{PaginatedRequest, PaginatedResponse, collect_all};
    use crate::error::Error;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeRequest {
        page_token: Option<String>,
    }

    impl PaginatedRequest for FakeRequest {
        fn with_page_token(&self, page_token: Option<String>) -> Self {
            Self { page_token }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FakeResponse {
        items: Vec<&'static str>,
        next_page_token: Option<String>,
        clear_calls: usize,
        merge_failure: Option<&'static str>,
    }

    impl PaginatedResponse for FakeResponse {
        fn next_page_token(&self) -> Option<&str> {
            self.next_page_token.as_deref()
        }

        fn merge_page(&mut self, next: Self) -> Result<(), Error> {
            if let Some(message) = next.merge_failure {
                return Err(Error::InvalidRequest(format!(
                    "pagination contract violation: {message}"
                )));
            }

            self.items.extend(next.items);
            self.next_page_token = next.next_page_token;
            self.clear_calls += next.clear_calls;
            Ok(())
        }

        fn clear_next_page_token(&mut self) {
            self.clear_calls += 1;
            self.next_page_token = None;
        }
    }

    #[tokio::test]
    async fn collect_all_merges_pages_and_clears_next_page_token() {
        let response = collect_all(
            FakeRequest { page_token: None },
            |request| async move {
                match request.page_token.as_deref() {
                    None => Ok(FakeResponse {
                        items: vec!["AAPL", "MSFT"],
                        next_page_token: Some("cursor-2".to_owned()),
                        clear_calls: 0,
                        merge_failure: None,
                    }),
                    Some("cursor-2") => Ok(FakeResponse {
                        items: vec!["TSLA"],
                        next_page_token: None,
                        clear_calls: 0,
                        merge_failure: None,
                    }),
                    other => panic!("unexpected page token: {other:?}"),
                }
            },
        )
        .await
        .expect("pagination should succeed");

        assert_eq!(response.items, vec!["AAPL", "MSFT", "TSLA"]);
        assert_eq!(response.next_page_token(), None);
        assert_eq!(response.clear_calls, 1);
    }

    #[tokio::test]
    async fn collect_all_rejects_repeated_next_page_tokens() {
        let second_page_fetches = Arc::new(AtomicUsize::new(0));
        let second_page_fetches_for_closure = Arc::clone(&second_page_fetches);

        let error = collect_all(
            FakeRequest { page_token: None },
            move |request| {
                let second_page_fetches = Arc::clone(&second_page_fetches_for_closure);

                async move {
                    match request.page_token.as_deref() {
                        None => Ok(FakeResponse {
                            items: vec!["AAPL"],
                            next_page_token: Some("cursor-2".to_owned()),
                            clear_calls: 0,
                            merge_failure: None,
                        }),
                        Some("cursor-2")
                            if second_page_fetches.fetch_add(1, Ordering::SeqCst) == 0 =>
                        {
                            Ok(FakeResponse {
                                items: vec!["MSFT"],
                                next_page_token: Some("cursor-2".to_owned()),
                                clear_calls: 0,
                                merge_failure: None,
                            })
                        }
                        Some("cursor-2") => Err(Error::InvalidRequest(
                            "pagination contract violation: unexpected extra fetch".to_owned(),
                        )),
                        other => panic!("unexpected page token: {other:?}"),
                    }
                }
            },
        )
        .await
        .expect_err("repeated tokens should fail");

        match error {
            Error::InvalidRequest(message) => {
                assert!(message.contains("pagination contract violation:"));
                assert!(message.contains("cursor-2"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn collect_all_propagates_merge_contract_failures() {
        let error = collect_all(
            FakeRequest { page_token: None },
            |request| async move {
                match request.page_token.as_deref() {
                    None => Ok(FakeResponse {
                        items: vec!["AAPL"],
                        next_page_token: Some("cursor-2".to_owned()),
                        clear_calls: 0,
                        merge_failure: None,
                    }),
                    Some("cursor-2") => Ok(FakeResponse {
                        items: vec!["MSFT"],
                        next_page_token: None,
                        clear_calls: 0,
                        merge_failure: Some("merge rejected next page"),
                    }),
                    other => panic!("unexpected page token: {other:?}"),
                }
            },
        )
        .await
        .expect_err("merge contract failures should propagate");

        assert_eq!(
            error,
            Error::InvalidRequest(
                "pagination contract violation: merge rejected next page".to_owned()
            )
        );
    }
}
