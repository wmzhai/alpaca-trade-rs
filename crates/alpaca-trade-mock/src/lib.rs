pub mod admin;
pub mod app;
pub mod handlers;
pub mod routes;
pub mod state;

use tokio::{net::TcpListener, task::JoinHandle};

pub use app::build_app;

#[derive(Debug)]
pub struct TestServer {
    pub base_url: String,
    _task: JoinHandle<()>,
}

pub async fn spawn_test_server() -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("local addr should exist");
    let app = build_app();

    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server should run");
    });

    TestServer {
        base_url: format!("http://{address}"),
        _task: task,
    }
}
