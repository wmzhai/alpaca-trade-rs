use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderName, RETRY_AFTER};
use serde::de::DeserializeOwned;

use crate::auth::Auth;
use crate::error::{Error, ErrorMeta};
use crate::observer::{
    ErrorEvent, NoopObserver, Observer, RequestStart, ResponseEvent, RetryEvent,
};
use crate::retry::RetryPolicy;
use crate::transport::endpoint::Endpoint;
use crate::transport::request::{NoContent, RequestParts};

const MAX_BODY_SNIPPET_CHARS: usize = 256;
const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone)]
pub(crate) struct HttpClient {
    client: reqwest::Client,
    retry_policy: RetryPolicy,
    observer: Arc<dyn Observer>,
}

impl HttpClient {
    pub(crate) fn new(timeout: Duration) -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(Error::from_reqwest)?;

        Ok(Self::with_client(
            client,
            RetryPolicy::trading_safe(),
            Arc::new(NoopObserver),
        ))
    }

    pub(crate) fn with_client(
        client: reqwest::Client,
        retry_policy: RetryPolicy,
        observer: Arc<dyn Observer>,
    ) -> Self {
        Self {
            client,
            retry_policy,
            observer,
        }
    }

    pub(crate) async fn get_json<T>(
        &self,
        base_url: &str,
        endpoint: Endpoint,
        auth: &Auth,
        query: Vec<(String, String)>,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.send_json(base_url, &endpoint, auth, RequestParts::with_query(query))
            .await
    }

    pub(crate) async fn send_json<T>(
        &self,
        base_url: &str,
        endpoint: &Endpoint,
        auth: &Auth,
        request: RequestParts,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let response = self.send(base_url, endpoint, auth, request).await?;
        let body_snippet = bounded_body_snippet(&response.body);

        serde_json::from_str(&response.body).map_err(|error| {
            let error = Error::Deserialize {
                message: error.to_string(),
                meta: self.error_meta(
                    endpoint,
                    Some(response.status),
                    response.request_id.clone(),
                    response.retry_after.clone(),
                    body_snippet,
                ),
            };
            self.notify_error(endpoint, &error);
            error
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn send_no_content(
        &self,
        base_url: &str,
        endpoint: &Endpoint,
        auth: &Auth,
        request: RequestParts,
    ) -> Result<NoContent, Error> {
        let response = self.send(base_url, endpoint, auth, request).await?;

        if response.status != StatusCode::NO_CONTENT {
            let error = Error::HttpStatus(self.error_meta(
                endpoint,
                Some(response.status),
                response.request_id.clone(),
                response.retry_after.clone(),
                bounded_body_snippet(&response.body),
            ));
            self.notify_error(endpoint, &error);
            return Err(error);
        }

        Ok(NoContent)
    }

    async fn send(
        &self,
        base_url: &str,
        endpoint: &Endpoint,
        auth: &Auth,
        request: RequestParts,
    ) -> Result<ResponseParts, Error> {
        let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint.path());
        let endpoint_name = endpoint.name().to_owned();
        let method = endpoint.method();
        let method_name = method.as_str().to_owned();
        let mut attempt = 1;

        loop {
            self.observer.on_request_start(&RequestStart {
                endpoint: endpoint_name.clone(),
                method: method_name.clone(),
                url: url.clone(),
            });

            let request_builder = self.build_request(&url, endpoint, auth, &request)?;
            let response = match request_builder.send().await {
                Ok(response) => response,
                Err(error) => {
                    let error = Error::from_reqwest_with_meta(
                        error,
                        Some(self.error_meta(endpoint, None, None, None, None)),
                    );
                    self.notify_error(endpoint, &error);
                    return Err(error);
                }
            };

            let status = response.status();
            let request_id = header_value(response.headers(), &X_REQUEST_ID);
            let retry_after = header_value(response.headers(), &RETRY_AFTER);

            if self
                .retry_policy
                .should_retry(&method, Some(status), attempt)
            {
                let wait_ms = parse_retry_after_ms(retry_after.as_deref())
                    .unwrap_or_else(|| self.retry_policy.wait_ms(attempt));
                self.observer.on_retry(&RetryEvent {
                    endpoint: endpoint_name.clone(),
                    method: method_name.clone(),
                    attempt,
                    status: Some(status.as_u16()),
                    wait_ms,
                });
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                attempt += 1;
                continue;
            }

            let body = response.text().await.map_err(|error| {
                let error = Error::from_reqwest_with_meta(
                    error,
                    Some(self.error_meta(
                        endpoint,
                        Some(status),
                        request_id.clone(),
                        retry_after.clone(),
                        None,
                    )),
                );
                self.notify_error(endpoint, &error);
                error
            })?;

            if status.is_success() {
                self.observer.on_response(&ResponseEvent {
                    endpoint: endpoint_name.clone(),
                    method: method_name.clone(),
                    status: status.as_u16(),
                    request_id: request_id.clone(),
                });

                return Ok(ResponseParts {
                    status,
                    request_id,
                    retry_after,
                    body,
                });
            }

            let error = if status == StatusCode::TOO_MANY_REQUESTS {
                Error::RateLimited(self.error_meta(
                    endpoint,
                    Some(status),
                    request_id.clone(),
                    retry_after.clone(),
                    bounded_body_snippet(&body),
                ))
            } else {
                Error::HttpStatus(self.error_meta(
                    endpoint,
                    Some(status),
                    request_id.clone(),
                    retry_after.clone(),
                    bounded_body_snippet(&body),
                ))
            };

            self.notify_error(endpoint, &error);
            return Err(error);
        }
    }

    fn build_request(
        &self,
        url: &str,
        endpoint: &Endpoint,
        auth: &Auth,
        request: &RequestParts,
    ) -> Result<reqwest::RequestBuilder, Error> {
        let mut request_builder = self.client.request(endpoint.method(), url);
        request_builder = request_builder.query(&request.query);

        if let Some(json_body) = &request.json_body {
            request_builder = request_builder.json(json_body);
        }

        if endpoint.requires_auth() {
            request_builder = auth.apply(request_builder)?;
        }

        Ok(request_builder)
    }

    fn error_meta(
        &self,
        endpoint: &Endpoint,
        status: Option<StatusCode>,
        request_id: Option<String>,
        retry_after: Option<String>,
        body: Option<String>,
    ) -> ErrorMeta {
        ErrorMeta {
            endpoint: endpoint.name().to_owned(),
            method: endpoint.method().as_str().to_owned(),
            status: status.map(|status| status.as_u16()),
            request_id,
            retry_after,
            body,
        }
    }

    fn notify_error(&self, endpoint: &Endpoint, error: &Error) {
        let meta = error.meta();
        self.observer.on_error(&ErrorEvent {
            endpoint: endpoint.name().to_owned(),
            method: endpoint.method().as_str().to_owned(),
            status: meta.and_then(|meta| meta.status),
            request_id: meta.and_then(|meta| meta.request_id.clone()),
        });
    }
}

impl fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.client;
        let _ = &self.retry_policy;
        let _ = &self.observer;
        f.debug_struct("HttpClient").finish_non_exhaustive()
    }
}

struct ResponseParts {
    status: StatusCode,
    request_id: Option<String>,
    retry_after: Option<String>,
    body: String,
}

fn bounded_body_snippet(body: &str) -> Option<String> {
    if body.is_empty() {
        return None;
    }

    Some(body.chars().take(MAX_BODY_SNIPPET_CHARS).collect())
}

fn header_value(headers: &HeaderMap, name: &HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn parse_retry_after_ms(value: Option<&str>) -> Option<u64> {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1000))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::HttpClient;

    struct ScriptedServer {
        base_url: String,
    }

    impl ScriptedServer {
        fn spawn(responses: Vec<String>) -> Self {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("listener should bind");
            let addr = listener
                .local_addr()
                .expect("listener should expose local addr");

            std::thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) =
                        listener.accept().expect("server should accept connection");
                    let mut buffer = [0_u8; 8192];
                    let _ = std::io::Read::read(&mut stream, &mut buffer)
                        .expect("server should read request");
                    std::io::Write::write_all(&mut stream, response.as_bytes())
                        .expect("server should write response");
                }
            });

            Self {
                base_url: format!("http://{addr}"),
            }
        }
    }

    #[tokio::test]
    async fn send_no_content_accepts_delete_204() {
        let server = ScriptedServer::spawn(vec![
            "HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".to_owned(),
        ]);
        let auth = crate::auth::Auth::new(Some("key".to_owned()), Some("secret".to_owned()))
            .expect("auth should build");
        let client = HttpClient::with_client(
            reqwest::Client::new(),
            crate::RetryPolicy::trading_safe(),
            Arc::new(crate::NoopObserver),
        );

        let result: Result<crate::transport::request::NoContent, crate::Error> = client
            .send_no_content(
                &server.base_url,
                &crate::transport::endpoint::Endpoint::new(
                    "positions.close_single",
                    reqwest::Method::DELETE,
                    "/v2/positions/AAPL",
                    true,
                ),
                &auth,
                crate::transport::request::RequestParts {
                    query: Vec::new(),
                    json_body: None,
                },
            )
            .await;

        assert!(result.is_ok(), "204 delete should succeed: {result:?}");
    }

    #[tokio::test]
    async fn http_status_error_captures_endpoint_request_id_and_body_snippet() {
        let server = ScriptedServer::spawn(vec![
            "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-122\r\ncontent-length: 27\r\nconnection: close\r\n\r\nservice offline for testing"
                .to_owned(),
            "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-123\r\ncontent-length: 27\r\nconnection: close\r\n\r\nservice offline for testing"
                .to_owned(),
        ]);
        let auth = crate::auth::Auth::new(Some("key".to_owned()), Some("secret".to_owned()))
            .expect("auth should build");
        let client = HttpClient::with_client(
            reqwest::Client::new(),
            crate::RetryPolicy::trading_safe(),
            Arc::new(crate::NoopObserver),
        );

        let error = client
            .send_json::<serde_json::Value>(
                &server.base_url,
                &crate::transport::endpoint::Endpoint::account_get(),
                &auth,
                crate::transport::request::RequestParts {
                    query: Vec::new(),
                    json_body: None,
                },
            )
            .await
            .expect_err("503 must fail");

        match error {
            crate::Error::HttpStatus(meta) => {
                assert_eq!(meta.endpoint, "account.get");
                assert_eq!(meta.method, "GET");
                assert_eq!(meta.status, Some(503));
                assert_eq!(meta.request_id.as_deref(), Some("req-123"));
                assert_eq!(meta.body.as_deref(), Some("service offline for testing"));
            }
            other => panic!("expected http status error, got {other:?}"),
        }
    }
}
