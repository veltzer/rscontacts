use clap::{CommandFactory, Parser, Subcommand};
use google_people1::api::ListConnectionsResponse;
use google_people1::{FieldMask, PeopleService};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

fn config_dir() -> PathBuf {
    let mut dir = dirs::home_dir().expect("Could not determine home directory");
    dir.push(".config");
    dir.push("rscontacts");
    std::fs::create_dir_all(&dir).expect("Could not create config directory");
    dir
}

#[derive(Parser)]
#[command(name = "rscontacts")]
#[command(about = "Google Contacts CLI tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Google (opens browser for OAuth2 consent)
    Auth {
        /// Don't open browser automatically; print URL instead
        #[arg(long)]
        no_browser: bool,
    },
    /// List all contacts
    List,
    /// Print contacts with non-English names
    CheckEnglish,
    /// Generate shell completions
    Complete {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

fn credentials_path() -> PathBuf {
    let path = config_dir().join("credentials.json");
    if !path.exists() {
        eprintln!("Error: credentials.json not found at {}", path.display());
        eprintln!("Download OAuth2 credentials from Google Cloud Console and place them there.");
        std::process::exit(1);
    }
    path
}

fn token_cache_path() -> PathBuf {
    config_dir().join("token_cache.json")
}

fn extract_access_token(token_info: &serde_json::Value) -> Option<String> {
    token_info
        .as_array()?
        .first()?
        .get("token")?
        .get("access_token")?
        .as_str()
        .map(String::from)
}

fn is_english_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii() || c == '\u{200f}' || c == '\u{200e}')
}

struct BrowserFlowDelegate;

impl yup_oauth2::authenticator_delegate::InstalledFlowDelegate for BrowserFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        _need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            if let Err(e) = open::that(url) {
                eprintln!("Failed to open browser: {}. Please open this URL manually:\n{}", e, url);
            }
            Ok(String::new())
        })
    }
}

fn build_connector() -> Result<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, Box<dyn std::error::Error>> {
    Ok(hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http2()
        .build())
}

async fn build_hub() -> Result<PeopleService<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>, Box<dyn std::error::Error>> {
    let cache_path = token_cache_path();
    if !cache_path.exists() {
        eprintln!("Error: not authenticated. Run 'rscontacts auth' first.");
        std::process::exit(1);
    }

    let data = std::fs::read_to_string(&cache_path)?;
    let token_info: serde_json::Value = serde_json::from_str(&data)?;

    let access_token = extract_access_token(&token_info)
        .ok_or("Invalid token cache. Run 'rscontacts auth' again.")?;

    let auth = yup_oauth2::AccessTokenAuthenticator::builder(access_token)
        .build()
        .await?;

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(build_connector()?);

    Ok(PeopleService::new(client, auth))
}

async fn cmd_auth(no_browser: bool) -> Result<(), Box<dyn std::error::Error>> {
    let secret = yup_oauth2::read_application_secret(credentials_path()).await?;

    let mut builder = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(token_cache_path());

    if !no_browser {
        builder = builder.flow_delegate(Box::new(BrowserFlowDelegate));
    }

    let auth = builder.build().await?;

    // Actually request a token so it gets persisted
    let scopes = &["https://www.googleapis.com/auth/contacts"];
    let _token = auth.token(scopes).await?;

    eprintln!("Authentication successful. Token cached to {}", token_cache_path().display());
    Ok(())
}

async fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(&["names", "emailAddresses", "phoneNumbers"]));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            for person in &connections {
                let name = person
                    .names
                    .as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.display_name.as_deref())
                    .unwrap_or("<no name>");

                let email = person
                    .email_addresses
                    .as_ref()
                    .and_then(|emails| emails.first())
                    .and_then(|e| e.value.as_deref())
                    .unwrap_or("");

                let phone = person
                    .phone_numbers
                    .as_ref()
                    .and_then(|phones| phones.first())
                    .and_then(|p| p.value.as_deref())
                    .unwrap_or("");

                if !email.is_empty() || !phone.is_empty() {
                    println!("{} | {} | {}", name, email, phone);
                } else {
                    println!("{}", name);
                }
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(())
}

async fn cmd_check_english() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(&["names", "emailAddresses"]));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            for person in &connections {
                let name = person
                    .names
                    .as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.display_name.as_deref())
                    .unwrap_or("");

                if !name.is_empty() && !is_english_name(name) {
                    let email = person
                        .email_addresses
                        .as_ref()
                        .and_then(|emails| emails.first())
                        .and_then(|e| e.value.as_deref())
                        .unwrap_or("");

                    if !email.is_empty() {
                        println!("{} | {}", name, email);
                    } else {
                        println!("{}", name);
                    }
                }
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default CryptoProvider");

    let cli = Cli::parse();

    match cli.command {
        Commands::Auth { no_browser } => cmd_auth(no_browser).await?,
        Commands::List => cmd_list().await?,
        Commands::CheckEnglish => cmd_check_english().await?,
        Commands::Complete { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "rscontacts", &mut std::io::stdout());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_auth_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "auth"]);
        assert!(matches!(cli.command, Commands::Auth { no_browser: false }));
    }

    #[test]
    fn test_cli_auth_no_browser() {
        let cli = Cli::parse_from(["rscontacts", "auth", "--no-browser"]);
        assert!(matches!(cli.command, Commands::Auth { no_browser: true }));
    }

    #[test]
    fn test_cli_list_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "list"]);
        assert!(matches!(cli.command, Commands::List));
    }

    #[test]
    fn test_cli_no_subcommand_fails() {
        assert!(Cli::try_parse_from(["rscontacts"]).is_err());
    }

    #[test]
    fn test_cli_unknown_subcommand_fails() {
        assert!(Cli::try_parse_from(["rscontacts", "foo"]).is_err());
    }

    #[test]
    fn test_extract_access_token_valid() {
        let json = serde_json::json!([
            {
                "scopes": ["https://www.googleapis.com/auth/contacts"],
                "token": {
                    "access_token": "ya29.test_token",
                    "refresh_token": "1//refresh",
                    "expires_at": [2026, 64, 8, 3, 7, 0, 0, 0, 0],
                    "id_token": null
                }
            }
        ]);
        assert_eq!(
            extract_access_token(&json),
            Some("ya29.test_token".to_string())
        );
    }

    #[test]
    fn test_extract_access_token_empty_array() {
        let json = serde_json::json!([]);
        assert_eq!(extract_access_token(&json), None);
    }

    #[test]
    fn test_extract_access_token_not_array() {
        let json = serde_json::json!({"token": "foo"});
        assert_eq!(extract_access_token(&json), None);
    }

    #[test]
    fn test_extract_access_token_missing_token_field() {
        let json = serde_json::json!([{"scopes": []}]);
        assert_eq!(extract_access_token(&json), None);
    }

    #[test]
    fn test_cli_check_english_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-english"]);
        assert!(matches!(cli.command, Commands::CheckEnglish));
    }

    #[test]
    fn test_is_english_name_ascii() {
        assert!(is_english_name("John Doe"));
        assert!(is_english_name("O'Brien"));
        assert!(is_english_name("Jean-Pierre"));
    }

    #[test]
    fn test_is_english_name_hebrew() {
        assert!(!is_english_name("יוסי כהן"));
    }

    #[test]
    fn test_is_english_name_arabic() {
        assert!(!is_english_name("محمد"));
    }

    #[test]
    fn test_is_english_name_chinese() {
        assert!(!is_english_name("张伟"));
    }

    #[test]
    fn test_is_english_name_mixed() {
        assert!(!is_english_name("John דוד"));
    }

    #[test]
    fn test_is_english_name_empty() {
        assert!(is_english_name(""));
    }

    #[test]
    fn test_config_dir_ends_with_rscontacts() {
        let dir = config_dir();
        assert!(dir.ends_with("rscontacts"));
        assert!(dir.to_str().unwrap().contains(".config"));
    }

    #[test]
    fn test_token_cache_path_is_in_config_dir() {
        let path = token_cache_path();
        assert!(path.ends_with("token_cache.json"));
        assert!(path.to_str().unwrap().contains(".config/rscontacts"));
    }
}
