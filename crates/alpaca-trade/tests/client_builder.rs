use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use alpaca_trade::{Client, Error, Observer, RequestStart, ResponseEvent, RetryPolicy};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

const OFFICIAL_API_KEY_ENV: &str = "APCA_API_KEY_ID";
const OFFICIAL_SECRET_KEY_ENV: &str = "APCA_API_SECRET_KEY";

#[derive(Debug)]
struct CapturedRequest {
    request_line: String,
    headers: HashMap<String, String>,
}

struct TestServer {
    base_url: String,
    request_rx: Receiver<CapturedRequest>,
    handle: thread::JoinHandle<()>,
}

impl TestServer {
    fn spawn(response: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let (request_tx, request_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("server should accept connection");
            let mut buffer = [0_u8; 8192];
            let bytes_read = stream
                .read(&mut buffer)
                .expect("server should read request");
            let request = String::from_utf8_lossy(&buffer[..bytes_read]).into_owned();
            let mut lines = request.split("\r\n");
            let request_line = lines
                .next()
                .expect("request should contain a request line")
                .to_owned();
            let mut headers = HashMap::new();

            for line in lines.take_while(|line| !line.is_empty()) {
                if let Some((name, value)) = line.split_once(':') {
                    headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
                }
            }

            request_tx
                .send(CapturedRequest {
                    request_line,
                    headers,
                })
                .expect("request should be captured");

            stream
                .write_all(response.as_bytes())
                .expect("server should write response");
        });

        Self {
            base_url: format!("http://{addr}"),
            request_rx,
            handle,
        }
    }

    fn into_request(self) -> CapturedRequest {
        let request = self
            .request_rx
            .recv()
            .expect("test should capture exactly one request");
        self.handle.join().expect("server thread should finish");
        request
    }
}

#[derive(Clone, Default)]
struct RecordingObserver {
    events: Arc<Mutex<Vec<ObservedEvent>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ObservedEvent {
    Start {
        endpoint: String,
        method: String,
        url: String,
    },
    Response {
        endpoint: String,
        method: String,
        status: u16,
        request_id: Option<String>,
    },
}

impl RecordingObserver {
    fn snapshot(&self) -> Vec<ObservedEvent> {
        self.events.lock().expect("observer lock").clone()
    }
}

impl Observer for RecordingObserver {
    fn on_request_start(&self, event: &RequestStart) {
        self.events
            .lock()
            .expect("observer lock")
            .push(ObservedEvent::Start {
                endpoint: event.endpoint.clone(),
                method: event.method.clone(),
                url: event.url.clone(),
            });
    }

    fn on_response(&self, event: &ResponseEvent) {
        self.events
            .lock()
            .expect("observer lock")
            .push(ObservedEvent::Response {
                endpoint: event.endpoint.clone(),
                method: event.method.clone(),
                status: event.status,
                request_id: event.request_id.clone(),
            });
    }
}

fn success_response() -> String {
    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nx-request-id: req-builder-1\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        account_json().len(),
        account_json()
    )
}

fn unavailable_response() -> String {
    "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
        .to_owned()
}

fn account_json() -> &'static str {
    r#"{"id":"acct-1","account_number":"010203ABCD","status":"ACTIVE"}"#
}

fn env_lock() -> &'static Mutex<()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

fn with_env_vars<R>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> R) -> R {
    let _guard = env_lock().lock().expect("env lock");
    let saved = vars
        .iter()
        .map(|(name, _)| ((*name).to_owned(), std::env::var(name).ok()))
        .collect::<Vec<_>>();

    for (name, value) in vars {
        match value {
            Some(value) => {
                // Tests serialize env mutation with a process-wide lock.
                unsafe { std::env::set_var(name, value) };
            }
            None => {
                // Tests serialize env mutation with a process-wide lock.
                unsafe { std::env::remove_var(name) };
            }
        }
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    for (name, value) in saved {
        match value {
            Some(value) => {
                // Restore the previous process env before releasing the lock.
                unsafe { std::env::set_var(name, value) };
            }
            None => {
                // Restore the previous process env before releasing the lock.
                unsafe { std::env::remove_var(name) };
            }
        }
    }

    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[test]
fn builder_builds_paper_client_by_default() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("paper client should build");

    let _ = client.account();
}

#[test]
fn builder_rejects_partial_credentials() {
    let error = Client::builder()
        .api_key("key-only")
        .build()
        .expect_err("partial credentials must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("api_key") && message.contains("secret_key")
    ));
}

#[test]
fn builder_rejects_missing_credentials() {
    let error = Client::builder()
        .build()
        .expect_err("missing credentials must fail");

    assert!(matches!(error, Error::MissingCredentials));
}

#[test]
fn builder_rejects_whitespace_only_credentials() {
    let error = Client::builder()
        .api_key("   ")
        .secret_key("\t")
        .build()
        .expect_err("blank credentials must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("api_key") || message.contains("secret_key")
    ));
}

#[test]
fn builder_rejects_invalid_base_url() {
    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url("not a url")
        .build()
        .expect_err("invalid base_url must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message) if message.contains("base_url")
    ));
}

#[test]
fn client_exposes_account_accessor() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("client should build");

    let _ = client.account();
}

#[tokio::test]
async fn builder_loads_credentials_from_official_env_names() {
    let server = TestServer::spawn(success_response());
    let client = with_env_vars(
        &[
            (OFFICIAL_API_KEY_ENV, Some("env-key")),
            (OFFICIAL_SECRET_KEY_ENV, Some("env-secret")),
        ],
        || {
            Client::builder()
                .credentials_from_env()
                .base_url(server.base_url.clone())
                .build()
                .expect("client should build from env")
        },
    );

    let account = client
        .account()
        .get()
        .await
        .expect("request with env credentials should succeed");
    assert_eq!(account.id, "acct-1");

    let request = server.into_request();
    assert_eq!(
        request.headers.get("apca-api-key-id"),
        Some(&"env-key".to_owned())
    );
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"env-secret".to_owned())
    );
}

#[tokio::test]
async fn builder_loads_credentials_from_custom_env_names() {
    let server = TestServer::spawn(success_response());
    let client = with_env_vars(
        &[
            ("CUSTOM_APCA_KEY", Some("custom-env-key")),
            ("CUSTOM_APCA_SECRET", Some("custom-env-secret")),
        ],
        || {
            Client::builder()
                .credentials_from_env_names("CUSTOM_APCA_KEY", "CUSTOM_APCA_SECRET")
                .base_url(server.base_url.clone())
                .build()
                .expect("client should build from custom env names")
        },
    );

    let account = client
        .account()
        .get()
        .await
        .expect("request with custom env credentials should succeed");
    assert_eq!(account.id, "acct-1");

    let request = server.into_request();
    assert_eq!(
        request.headers.get("apca-api-key-id"),
        Some(&"custom-env-key".to_owned())
    );
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"custom-env-secret".to_owned())
    );
}

#[tokio::test]
async fn credentials_from_env_clears_explicit_credentials() {
    let server = TestServer::spawn(success_response());
    let client = with_env_vars(
        &[
            (OFFICIAL_API_KEY_ENV, Some("env-key")),
            (OFFICIAL_SECRET_KEY_ENV, Some("env-secret")),
        ],
        || {
            Client::builder()
                .api_key("explicit-key")
                .secret_key("explicit-secret")
                .credentials_from_env()
                .base_url(server.base_url.clone())
                .build()
                .expect("env mode should override explicit credentials")
        },
    );

    client
        .account()
        .get()
        .await
        .expect("env-backed request should succeed");

    let request = server.into_request();
    assert_eq!(
        request.headers.get("apca-api-key-id"),
        Some(&"env-key".to_owned())
    );
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"env-secret".to_owned())
    );
}

#[tokio::test]
async fn explicit_credentials_override_env_mode_when_set_last() {
    let server = TestServer::spawn(success_response());
    let client = with_env_vars(
        &[
            (OFFICIAL_API_KEY_ENV, Some("env-key")),
            (OFFICIAL_SECRET_KEY_ENV, Some("env-secret")),
        ],
        || {
            Client::builder()
                .credentials_from_env()
                .api_key("explicit-key")
                .secret_key("explicit-secret")
                .base_url(server.base_url.clone())
                .build()
                .expect("explicit credentials should override env mode")
        },
    );

    client
        .account()
        .get()
        .await
        .expect("request with explicit override should succeed");

    let request = server.into_request();
    assert_eq!(
        request.headers.get("apca-api-key-id"),
        Some(&"explicit-key".to_owned())
    );
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"explicit-secret".to_owned())
    );
}

#[tokio::test]
async fn builder_uses_injected_reqwest_client_default_headers() {
    let server = TestServer::spawn(success_response());
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        HeaderName::from_static("x-builder-default"),
        HeaderValue::from_static("from-injected-client"),
    );
    let reqwest_client = reqwest::Client::builder()
        .default_headers(default_headers)
        .build()
        .expect("reqwest client should build");

    let account = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .reqwest_client(reqwest_client)
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect("request should succeed");

    assert_eq!(account.id, "acct-1");

    let request = server.into_request();
    assert_eq!(
        request.headers.get("x-builder-default"),
        Some(&"from-injected-client".to_owned())
    );
}

#[tokio::test]
async fn builder_observer_receives_success_lifecycle_callbacks() {
    let server = TestServer::spawn(success_response());
    let observer = RecordingObserver::default();

    let account = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .observer(observer.clone())
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect("request should succeed");

    assert_eq!(account.id, "acct-1");
    assert_eq!(
        observer.snapshot(),
        vec![
            ObservedEvent::Start {
                endpoint: "account.get".to_owned(),
                method: "GET".to_owned(),
                url: format!("{}/v2/account", server.base_url),
            },
            ObservedEvent::Response {
                endpoint: "account.get".to_owned(),
                method: "GET".to_owned(),
                status: 200,
                request_id: Some("req-builder-1".to_owned()),
            },
        ]
    );

    let request = server.into_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
}

#[tokio::test]
async fn builder_retry_policy_drives_transport_behavior() {
    let server = TestServer::spawn(unavailable_response());
    let mut retry_policy = RetryPolicy::trading_safe();
    retry_policy.max_get_attempts = 1;
    retry_policy.base_delay_ms = 0;

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .retry_policy(retry_policy)
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("custom retry policy should disable retry");

    match error {
        Error::HttpStatus(meta) => assert_eq!(meta.status, Some(503)),
        other => panic!("expected http status error, got {other:?}"),
    }

    let request = server.into_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
}
