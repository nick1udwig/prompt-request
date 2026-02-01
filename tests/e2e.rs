use std::net::SocketAddr;

use serde::Deserialize;

#[tokio::test]
async fn e2e_markdown_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("E2E").is_err() {
        eprintln!("E2E not set, skipping");
        return Ok(());
    }

    let cfg = prompt_request::config::Config::from_env()?;
    let state = prompt_request::build_state(&cfg).await?;
    let app = prompt_request::build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr: SocketAddr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let base = format!("http://{}", addr);
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct AccountResp {
        api_key: String,
    }

    let acct = client
        .post(format!("{}/api/accounts", base))
        .send()
        .await?;
    assert!(acct.status().is_success());
    let acct: AccountResp = acct.json().await?;

    #[derive(Deserialize)]
    struct CreateResp {
        uuid: String,
    }

    let create = client
        .post(format!("{}/api/requests", base))
        .header("Authorization", format!("Bearer {}", acct.api_key))
        .header("Content-Type", "text/markdown")
        .body("# Hello\n".to_string())
        .send()
        .await?;
    assert!(create.status().is_success());
    let create: CreateResp = create.json().await?;

    let raw = client
        .get(format!("{}/{}", base, create.uuid))
        .send()
        .await?
        .text()
        .await?;
    assert_eq!(raw, "# Hello\n");

    Ok(())
}
