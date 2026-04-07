use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let cache_path = rscontacts::helpers::token_cache_path();
    let secret = yup_oauth2::read_application_secret(rscontacts::helpers::credentials_path())
        .await
        .unwrap();

    eprintln!("Building authenticator...");
    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(cache_path)
    .flow_delegate(Box::new(rscontacts::helpers::NoInteractionDelegate))
    .build()
    .await
    .unwrap();

    // Test: get token for contacts (full, not readonly) - this was just refreshed by auth command
    eprintln!("Getting token for contacts scope...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        auth.token(&["https://www.googleapis.com/auth/contacts"]),
    ).await {
        Ok(Ok(tok)) => eprintln!("Got contacts token: {}...", &tok.token().unwrap_or("none")[..20]),
        Ok(Err(e)) => eprintln!("Error: {e}"),
        Err(_) => eprintln!("TIMED OUT after 10s"),
    }

    // Test: get token for contacts.readonly - this one may need refreshing
    eprintln!("Getting token for contacts.readonly scope...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        auth.token(&["https://www.googleapis.com/auth/contacts.readonly"]),
    ).await {
        Ok(Ok(tok)) => eprintln!("Got readonly token: {}...", &tok.token().unwrap_or("none")[..20]),
        Ok(Err(e)) => eprintln!("Error: {e}"),
        Err(_) => eprintln!("TIMED OUT after 10s"),
    }
}
