use clap::{Parser, Subcommand};
use google_people1::api::ListConnectionsResponse;
use google_people1::{FieldMask, PeopleService};
use std::path::PathBuf;

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
    Auth,
    /// List all contacts
    List,
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

async fn build_hub() -> Result<PeopleService<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>, Box<dyn std::error::Error>> {
    let secret = yup_oauth2::read_application_secret(credentials_path()).await?;

    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(token_cache_path())
    .build()
    .await?;

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()?
                .https_or_http()
                .enable_http2()
                .build(),
        );

    Ok(PeopleService::new(client, auth))
}

async fn cmd_auth() -> Result<(), Box<dyn std::error::Error>> {
    // Just trigger the OAuth flow — build_hub will open browser if no cached token
    let _hub = build_hub().await?;
    eprintln!("Authentication successful. Token cached to {}", token_cache_path().display());
    Ok(())
}

async fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut page_token: Option<String> = None;
    let mut total = 0;

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
                total += 1;
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    eprintln!("\nTotal contacts: {}", total);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default CryptoProvider");

    let cli = Cli::parse();

    match cli.command {
        Commands::Auth => cmd_auth().await?,
        Commands::List => cmd_list().await?,
    }

    Ok(())
}
