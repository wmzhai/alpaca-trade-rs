fn print_help() {
    println!(
        "alpaca-trade-mock

USAGE:
    alpaca-trade-mock [--bind <ADDR>]

OPTIONS:
    -h, --help    Print help information
        --bind    Bind address (default: 127.0.0.1:9817)"
    );
}

#[tokio::main]
async fn main() {
    let mut bind = String::from("127.0.0.1:9817");
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
