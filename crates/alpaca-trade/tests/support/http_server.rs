use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug)]
pub struct CapturedRequest {
    pub request_line: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct TestServer {
    base_url: String,
    request_rx: Receiver<CapturedRequest>,
    handle: thread::JoinHandle<()>,
}

impl TestServer {
    pub fn spawn(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should expose local addr");
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

                for line in lines.by_ref().take_while(|line| !line.is_empty()) {
                    if let Some((name, value)) = line.split_once(':') {
                        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
                    }
                }

                let body = lines.collect::<Vec<_>>().join("\r\n");
                request_tx
                    .send(CapturedRequest {
                        request_line,
                        headers,
                        body,
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

    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn into_single_request(self) -> CapturedRequest {
        let request = self.request_rx.recv().expect("request should be captured");
        self.handle.join().expect("server thread should finish");
        request
    }
}
