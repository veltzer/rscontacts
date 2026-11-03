//! Regression test: a failed token refresh must produce an error, not a hang.
//!
//! With `InstalledFlowReturnMethod::HTTPRedirect`, yup-oauth2 ignored the
//! non-interactive delegate's refusal and waited forever for a browser
//! callback, so every command hung once the refresh token expired. The token
//! endpoint here is a closed local port, so the refresh always fails and the
//! test runs offline.

use std::time::Duration;

use rscontacts::helpers::noninteractive_authenticator;

#[tokio::test]
async fn failed_refresh_errors_instead_of_hanging() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let dir = std::env::temp_dir().join(format!("rscontacts-auth-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let credentials = dir.join("credentials.json");
    let cache = dir.join("token_cache.json");
    std::fs::write(
        &credentials,
        r#"{"installed":{"client_id":"id","client_secret":"secret",
            "auth_uri":"https://127.0.0.1:1/auth","token_uri":"https://127.0.0.1:1/token",
            "redirect_uris":["http://localhost"]}}"#,
    )
    .unwrap();
    std::fs::write(
        &cache,
        r#"[{"scopes":["https://www.googleapis.com/auth/contacts"],
            "token":{"access_token":"expired","refresh_token":"refresh",
            "expires_at":[2020,1,0,0,0,0,0,0,0],"id_token":null}}]"#,
    )
    .unwrap();

    let auth = noninteractive_authenticator(&credentials, &cache)
        .await
        .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        auth.token(&["https://www.googleapis.com/auth/contacts"]),
    )
    .await;
    std::fs::remove_dir_all(&dir).unwrap();

    let outcome = result.expect("token() hung instead of failing");
    assert!(
        matches!(outcome, Err(yup_oauth2::Error::UserError(_))),
        "expected the delegate's refusal, got {outcome:?}"
    );
}
