#[tokio::main]
async fn main() {
    if let Err(err) = prompt_request::run().await {
        eprintln!("startup error: {err}");
        std::process::exit(1);
    }
}
