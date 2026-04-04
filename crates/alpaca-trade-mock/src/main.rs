fn print_help() {
    println!(
        "alpaca-trade-mock\n\nUSAGE:\n    alpaca-trade-mock [--bind <ADDR>]\n\nOPTIONS:\n    -h, --help    Print help information\n        --bind    Bind address (default: 127.0.0.1:16803)"
    );
}

#[tokio::main]
async fn main() {
    let mut bind = String::from("127.0.0.1:16803");
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--bind" => {
                bind = args.next().expect("--bind requires an address");
            }
            other => {
                eprintln!("unexpected argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .expect("listener should bind");
    axum::serve(listener, alpaca_trade_mock::build_app())
        .await
        .expect("server should run");
}
