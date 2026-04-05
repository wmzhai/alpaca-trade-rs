use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct CapturedRequest {
    pub request_line: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct TestServer {
    base_url: String,
    requests_rx: Receiver<Vec<CapturedRequest>>,
    handle: thread::JoinHandle<()>,
}

impl TestServer {
    pub fn spawn(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        listener
            .set_nonblocking(true)
            .expect("listener should support nonblocking mode");
        let addr = listener
            .local_addr()
            .expect("listener should expose local addr");
        let (requests_tx, requests_rx) = mpsc::sync_channel(1);

        let handle = thread::spawn(move || {
            let mut requests = Vec::new();

            for response in responses {
                let Some(mut stream) = accept_with_timeout(&listener, Duration::from_secs(2))
                else {
                    break;
                };
                requests.push(read_request(&mut stream));
                stream
                    .write_all(response.as_bytes())
                    .expect("server should write response");
            }

            send_requests(requests_tx, requests);
        });

        Self {
            base_url: format!("http://{addr}"),
            requests_rx,
            handle,
        }
    }

    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn into_requests(self) -> Vec<CapturedRequest> {
        let requests = self
            .requests_rx
            .recv()
            .expect("request capture should finish");
        self.handle.join().expect("server thread should finish");
        requests
    }

    pub fn into_single_request(self) -> CapturedRequest {
        let mut requests = self.into_requests();
        assert_eq!(requests.len(), 1, "expected exactly one captured request");
        requests.remove(0)
    }
}

fn accept_with_timeout(listener: &TcpListener, timeout: Duration) -> Option<TcpStream> {
    let deadline = Instant::now() + timeout;

    loop {
        match listener.accept() {
            Ok((stream, _)) => return Some(stream),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("server should accept connection: {error}"),
        }
    }
}

fn read_request(stream: &mut TcpStream) -> CapturedRequest {
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("stream should support read timeout");

    let mut raw = Vec::new();
    let mut buffer = [0_u8; 1024];
    let header_deadline = Instant::now() + Duration::from_secs(2);
    let header_end = loop {
        let bytes_read = match stream.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if Instant::now() >= header_deadline {
                    panic!("request headers should arrive before timeout");
                }
                continue;
            }
            Err(error) => panic!("server should read request: {error}"),
        };
        assert!(bytes_read > 0, "request should include headers");
        raw.extend_from_slice(&buffer[..bytes_read]);

        if let Some(header_end) = header_end_offset(&raw) {
            break header_end;
        }
    };

    let header_text = String::from_utf8_lossy(&raw[..header_end]).into_owned();
    let mut lines = header_text.split("\r\n");
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

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body_bytes = raw[header_end..].to_vec();
    let body_deadline = Instant::now() + Duration::from_secs(2);

    while body_bytes.len() < content_length {
        let bytes_read = match stream.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if Instant::now() >= body_deadline {
                    panic!("request body should arrive before timeout");
                }
                continue;
            }
            Err(error) => panic!("server should read request body: {error}"),
        };
        if bytes_read == 0 {
            break;
        }
        body_bytes.extend_from_slice(&buffer[..bytes_read]);
    }

    body_bytes.truncate(content_length);
    assert_eq!(
        body_bytes.len(),
        content_length,
        "request body should match content-length"
    );

    CapturedRequest {
        request_line,
        headers,
        body: String::from_utf8_lossy(&body_bytes).into_owned(),
    }
}

fn header_end_offset(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

fn send_requests(sender: SyncSender<Vec<CapturedRequest>>, requests: Vec<CapturedRequest>) {
    sender
        .send(requests)
        .expect("request capture should be delivered");
}
