use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use alpaca_trade::{Client, Error, Observer, RequestStart, ResponseEvent, RetryPolicy};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

const OFFICIAL_API_KEY_ENV: &str = "APCA_API_KEY_ID";
const OFFICIAL_SECRET_KEY_ENV: &str = "APCA_API_SECRET_KEY";
const CUSTOM_API_KEY_ENV: &str = "CUSTOM_APCA_KEY";
const CUSTOM_SECRET_KEY_ENV: &str = "CUSTOM_APCA_SECRET";
const SUBPROCESS_BASE_URL_ENV: &str = "ALPACA_TRADE_TEST_BASE_URL";

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

fn all_builder_env_names() -> [&'static str; 4] {
    [
        OFFICIAL_API_KEY_ENV,
        OFFICIAL_SECRET_KEY_ENV,
        CUSTOM_API_KEY_ENV,
        CUSTOM_SECRET_KEY_ENV,
    ]
}

fn run_subprocess_test(test_name: &str, envs: &[(&str, &str)]) {
    let mut command = Command::new(std::env::current_exe().expect("current test binary"));
    command.arg(test_name).arg("--exact").arg("--nocapture");

    for name in all_builder_env_names() {
        command.env_remove(name);
    }

    for (name, value) in envs {
        command.env(name, value);
    }

    let output = command.output().expect("subprocess should run");

    assert!(
        output.status.success(),
        "subprocess test `{test_name}` failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
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

#[test]
fn builder_loads_credentials_from_official_env_names() {
    let server = TestServer::spawn(success_response());
    run_subprocess_test(
        "subprocess_builder_loads_credentials_from_official_env_names",
        &[
            (SUBPROCESS_BASE_URL_ENV, server.base_url.as_str()),
            (OFFICIAL_API_KEY_ENV, "env-key"),
            (OFFICIAL_SECRET_KEY_ENV, "env-secret"),
        ],
    );

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
async fn subprocess_builder_loads_credentials_from_official_env_names() {
    let base_url = match std::env::var(SUBPROCESS_BASE_URL_ENV) {
        Ok(base_url) => base_url,
        Err(_) => return,
    };

    let account = Client::builder()
        .credentials_from_env()
        .base_url(base_url)
        .build()
        .expect("client should build from env")
        .account()
        .get()
        .await
        .expect("request with env credentials should succeed");

    assert_eq!(account.id, "acct-1");
}

#[test]
fn builder_loads_credentials_from_custom_env_names() {
    let server = TestServer::spawn(success_response());
    run_subprocess_test(
        "subprocess_builder_loads_credentials_from_custom_env_names",
        &[
            (SUBPROCESS_BASE_URL_ENV, server.base_url.as_str()),
            (CUSTOM_API_KEY_ENV, "custom-env-key"),
            (CUSTOM_SECRET_KEY_ENV, "custom-env-secret"),
        ],
    );

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
async fn subprocess_builder_loads_credentials_from_custom_env_names() {
    let base_url = match std::env::var(SUBPROCESS_BASE_URL_ENV) {
        Ok(base_url) => base_url,
        Err(_) => return,
    };

    let account = Client::builder()
        .credentials_from_env_names(CUSTOM_API_KEY_ENV, CUSTOM_SECRET_KEY_ENV)
        .base_url(base_url)
        .build()
        .expect("client should build from custom env names")
        .account()
        .get()
        .await
        .expect("request with custom env credentials should succeed");

    assert_eq!(account.id, "acct-1");
}

#[test]
fn credentials_from_env_clears_explicit_credentials() {
    let server = TestServer::spawn(success_response());
    run_subprocess_test(
        "subprocess_credentials_from_env_clears_explicit_credentials",
        &[
            (SUBPROCESS_BASE_URL_ENV, server.base_url.as_str()),
            (OFFICIAL_API_KEY_ENV, "env-key"),
            (OFFICIAL_SECRET_KEY_ENV, "env-secret"),
        ],
    );

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
async fn subprocess_credentials_from_env_clears_explicit_credentials() {
    let base_url = match std::env::var(SUBPROCESS_BASE_URL_ENV) {
        Ok(base_url) => base_url,
        Err(_) => return,
    };

    let account = Client::builder()
        .api_key("explicit-key")
        .secret_key("explicit-secret")
        .credentials_from_env()
        .base_url(base_url)
        .build()
        .expect("env mode should override explicit credentials")
        .account()
        .get()
        .await
        .expect("env-backed request should succeed");

    assert_eq!(account.id, "acct-1");
}

#[test]
fn explicit_credentials_override_env_mode_when_set_last() {
    let server = TestServer::spawn(success_response());
    run_subprocess_test(
        "subprocess_explicit_credentials_override_env_mode_when_set_last",
        &[
            (SUBPROCESS_BASE_URL_ENV, server.base_url.as_str()),
            (OFFICIAL_API_KEY_ENV, "env-key"),
            (OFFICIAL_SECRET_KEY_ENV, "env-secret"),
        ],
    );

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
async fn subprocess_explicit_credentials_override_env_mode_when_set_last() {
    let base_url = match std::env::var(SUBPROCESS_BASE_URL_ENV) {
        Ok(base_url) => base_url,
        Err(_) => return,
    };

    let account = Client::builder()
        .credentials_from_env()
        .api_key("explicit-key")
        .secret_key("explicit-secret")
        .base_url(base_url)
        .build()
        .expect("explicit credentials should override env mode")
        .account()
        .get()
        .await
        .expect("request with explicit override should succeed");

    assert_eq!(account.id, "acct-1");
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

#[test]
fn builder_rejects_custom_reqwest_client_after_non_default_timeout() {
    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .timeout(Duration::from_secs(5))
        .reqwest_client(reqwest::Client::new())
        .build()
        .expect_err("custom client with custom timeout must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("reqwest_client")
                && message.contains("timeout")
                && message.contains("provided reqwest::Client")
    ));
}

#[test]
fn builder_rejects_non_default_timeout_after_custom_reqwest_client() {
    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .reqwest_client(reqwest::Client::new())
        .timeout(Duration::from_secs(5))
        .build()
        .expect_err("custom timeout after custom client must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("reqwest_client")
                && message.contains("timeout")
                && message.contains("provided reqwest::Client")
    ));
}

#[test]
fn builder_rejects_default_timeout_after_custom_reqwest_client() {
    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .reqwest_client(reqwest::Client::new())
        .timeout(Duration::from_secs(30))
        .build()
        .expect_err("explicit default timeout after custom client must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("reqwest_client")
                && message.contains("timeout")
                && message.contains("provided reqwest::Client")
    ));
}

#[test]
fn builder_rejects_custom_reqwest_client_after_default_timeout() {
    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .timeout(Duration::from_secs(30))
        .reqwest_client(reqwest::Client::new())
        .build()
        .expect_err("custom client after explicit default timeout must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("reqwest_client")
                && message.contains("timeout")
                && message.contains("provided reqwest::Client")
    ));
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
async fn builder_observer_redacts_userinfo_from_request_start_url() {
    let server = TestServer::spawn(success_response());
    let observer = RecordingObserver::default();
    let secret_base_url = format!(
        "http://user:secret@{}",
        server.base_url.trim_start_matches("http://")
    );

    let account = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .observer(observer.clone())
        .base_url(secret_base_url)
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect("request should succeed");

    assert_eq!(account.id, "acct-1");

    let events = observer.snapshot();
    assert_eq!(events.len(), 2);
    let ObservedEvent::Start { url, .. } = &events[0] else {
        panic!("first event should be request start: {events:?}");
    };
    assert_eq!(url, &format!("{}/v2/account", server.base_url));
    assert!(!url.contains("secret"), "observer url leaked secret: {url}");
    assert!(
        !url.contains("user@"),
        "observer url leaked username: {url}"
    );
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
