use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use alpaca_trade::{Client, Error};

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
        Self::spawn_scripted(vec![response])
    }

    fn spawn_scripted(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let (request_tx, request_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            for response in responses {
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
            }
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

fn account_json() -> &'static str {
    r#"{"id":"acct-1","account_number":"010203ABCD","status":"ACTIVE"}"#
}

#[tokio::test]
async fn account_get_hits_official_path_and_sends_auth_headers() {
    let server = TestServer::spawn(format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        account_json().len(),
        account_json()
    ));

    let account = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect("account request should succeed");

    assert_eq!(account.id, "acct-1");

    let request = server.into_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
    assert_eq!(
        request.headers.get("apca-api-key-id"),
        Some(&"key".to_owned())
    );
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"secret".to_owned())
    );
}

#[tokio::test]
async fn account_get_maps_429_to_rate_limited() {
    let server = TestServer::spawn_scripted(vec![
        "HTTP/1.1 429 Too Many Requests\r\nretry-after: 0\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
        "HTTP/1.1 429 Too Many Requests\r\nretry-after: 17\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("429 response must fail");

    match error {
        Error::RateLimited(meta) => {
            assert_eq!(meta.retry_after.as_deref(), Some("17"));
            assert_eq!(meta.body.as_deref(), Some("slow down"));
        }
        other => panic!("expected rate limited error, got {other:?}"),
    }

    let request = server.into_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
}

#[tokio::test]
async fn account_get_maps_non_success_status_to_http_status() {
    let server = TestServer::spawn_scripted(vec![
        "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
        "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("503 response must fail");

    match error {
        Error::HttpStatus(meta) => {
            assert_eq!(meta.status, Some(503));
            assert_eq!(meta.body.as_deref(), Some("service offline"));
        }
        other => panic!("expected http status error, got {other:?}"),
    }

    let request = server.into_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
}

#[tokio::test]
async fn account_get_maps_malformed_json_to_deserialize() {
    let server = TestServer::spawn(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 15\r\nconnection: close\r\n\r\n{not valid json"
            .to_owned(),
    );

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("invalid json must fail");

    match error {
        Error::Deserialize { message, .. } => {
            assert!(!message.is_empty());
        }
        other => panic!("expected deserialize error, got {other:?}"),
    }

    let request = server.into_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
}
