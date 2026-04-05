use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use alpaca_trade::Client;

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

fn clock_json() -> &'static str {
    r#"{"timestamp":"2024-04-05T13:30:00Z","is_open":true,"next_open":"2024-04-08T13:30:00Z","next_close":"2024-04-05T20:00:00Z"}"#
}

#[tokio::test]
async fn clock_get_hits_official_path_and_sends_auth_headers() {
    let server = TestServer::spawn(format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        clock_json().len(),
        clock_json()
    ));

    let clock = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .clock()
        .get()
        .await
        .expect("clock request should succeed");

    assert_eq!(clock.timestamp, "2024-04-05T13:30:00Z");
    assert!(clock.is_open);

    let request = server.into_request();
    assert_eq!(request.request_line, "GET /v2/clock HTTP/1.1");
    assert_eq!(request.headers.get("apca-api-key-id"), Some(&"key".to_owned()));
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"secret".to_owned())
    );
}
