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
    CheckEnglish {
        /// Interactively fix each non-English contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
    },
    /// Print contacts with all-caps names
    CheckCaps {
        /// Interactively fix each all-caps contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
    },
    /// Print contacts with phone numbers missing a country code
    CheckPhone {
        /// Auto-fix by prepending country code
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
        /// Country code to prepend (without +)
        #[arg(long, default_value = "972")]
        country: String,
    },
    /// Remove dashes/minuses from phone numbers
    FixRemoveMinus {
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove whitespace from phone numbers
    FixPhoneRemoveWhitespace {
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print version information
    Version,
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

fn is_all_caps(name: &str) -> bool {
    let alpha_chars: String = name.chars().filter(|c| c.is_alphabetic()).collect();
    alpha_chars.len() >= 2 && alpha_chars == alpha_chars.to_uppercase()
}

fn is_english_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii() || c == '\u{200f}' || c == '\u{200e}')
}

fn is_fixable_phone(phone: &str) -> bool {
    let trimmed = phone.trim();
    let digits: Vec<char> = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    !trimmed.is_empty()
        && !trimmed.contains(|c: char| c.is_alphabetic())
        && !trimmed.starts_with('*')
        && digits.len() > 3
}

fn has_country_code(phone: &str) -> bool {
    let trimmed = phone.trim();
    trimmed.starts_with('+') || trimmed.starts_with("00")
}

fn add_country_code(phone: &str, country: &str) -> String {
    let trimmed = phone.trim();
    let without_leading_zero = trimmed.strip_prefix('0').unwrap_or(trimmed);
    format!("+{}{}", country, without_leading_zero)
}

struct NoInteractionDelegate;

impl yup_oauth2::authenticator_delegate::InstalledFlowDelegate for NoInteractionDelegate {
    fn present_user_url<'a>(
        &'a self,
        _url: &'a str,
        _need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            Err("Not authenticated. Run 'rscontacts auth' first.".to_string())
        })
    }
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

    let secret = yup_oauth2::read_application_secret(credentials_path()).await?;

    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(cache_path)
    .flow_delegate(Box::new(NoInteractionDelegate))
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

fn prompt_fix_action(_name: &str) -> Result<char, Box<dyn std::error::Error>> {
    use std::io::Write;
    loop {
        eprint!("  [r]ename / [d]elete / [s]kip? ");
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim().chars().next() {
            Some(c @ ('r' | 'd' | 's')) => return Ok(c),
            _ => eprintln!("  Invalid choice. Enter r, d, or s."),
        }
    }
}

fn prompt_new_name(old_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Write;
    eprint!("  New name for \"{}\": ", old_name);
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let new_name = input.trim().to_string();
    if new_name.is_empty() {
        return Err("Empty name not allowed".into());
    }
    Ok(new_name)
}

async fn cmd_check_english(fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    // Collect all non-English contacts first (to avoid pagination issues during mutation)
    let mut non_english: Vec<google_people1::api::Person> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(&["names", "emailAddresses", "metadata"]));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            for person in connections {
                let name = person
                    .names
                    .as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.display_name.as_deref())
                    .unwrap_or("");

                if !name.is_empty() && !is_english_name(name) {
                    non_english.push(person);
                }
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    for person in &non_english {
        let name = person
            .names
            .as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.display_name.as_deref())
            .unwrap_or("");

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

        if fix {
            let resource_name = person
                .resource_name
                .as_deref()
                .ok_or("Contact missing resource name")?;

            match prompt_fix_action(name)? {
                'r' => {
                    let new_name = prompt_new_name(name)?;
                    let mut updated = person.clone();
                    if let Some(ref mut names) = updated.names {
                        if let Some(first) = names.first_mut() {
                            first.given_name = Some(new_name.clone());
                            first.family_name = None;
                            first.unstructured_name = Some(new_name.clone());
                        }
                    }
                    hub.people()
                        .update_contact(updated, resource_name)
                        .update_person_fields(FieldMask::new::<&str>(&["names"]))
                        .doit()
                        .await?;
                    eprintln!("  Renamed to \"{}\"", new_name);
                }
                'd' => {
                    hub.people().delete_contact(resource_name).doit().await?;
                    eprintln!("  Deleted.");
                }
                's' => {
                    eprintln!("  Skipped.");
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}

async fn cmd_check_caps(fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut all_caps: Vec<google_people1::api::Person> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(&["names", "emailAddresses", "metadata"]));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            for person in connections {
                let name = person
                    .names
                    .as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.display_name.as_deref())
                    .unwrap_or("");

                if !name.is_empty() && is_all_caps(name) {
                    all_caps.push(person);
                }
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    for person in &all_caps {
        let name = person
            .names
            .as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.display_name.as_deref())
            .unwrap_or("");

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

        if fix {
            let resource_name = person
                .resource_name
                .as_deref()
                .ok_or("Contact missing resource name")?;

            match prompt_fix_action(name)? {
                'r' => {
                    let new_name = prompt_new_name(name)?;
                    let mut updated = person.clone();
                    if let Some(ref mut names) = updated.names {
                        if let Some(first) = names.first_mut() {
                            first.given_name = Some(new_name.clone());
                            first.family_name = None;
                            first.unstructured_name = Some(new_name.clone());
                        }
                    }
                    hub.people()
                        .update_contact(updated, resource_name)
                        .update_person_fields(FieldMask::new::<&str>(&["names"]))
                        .doit()
                        .await?;
                    eprintln!("  Renamed to \"{}\"", new_name);
                }
                'd' => {
                    hub.people().delete_contact(resource_name).doit().await?;
                    eprintln!("  Deleted.");
                }
                's' => {
                    eprintln!("  Skipped.");
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}

async fn cmd_check_phone(fix: bool, dry_run: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut contacts: Vec<google_people1::api::Person> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(&["names", "phoneNumbers", "metadata"]));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            for person in connections {
                let has_bad_phone = person
                    .phone_numbers
                    .as_ref()
                    .is_some_and(|nums| nums.iter().any(|p| {
                        p.value.as_deref().is_some_and(|v| is_fixable_phone(v) && !has_country_code(v))
                    }));

                if has_bad_phone {
                    contacts.push(person);
                }
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    for person in &contacts {
        let name = person
            .names
            .as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.display_name.as_deref())
            .unwrap_or("<no name>");

        let phones: Vec<&str> = person
            .phone_numbers
            .as_ref()
            .map(|nums| nums.iter().filter_map(|p| p.value.as_deref()).collect())
            .unwrap_or_default();

        let bad_phones: Vec<&str> = phones
            .iter()
            .filter(|p| is_fixable_phone(p) && !has_country_code(p))
            .copied()
            .collect();

        for phone in &bad_phones {
            let fixed = add_country_code(phone, country);
            if fix || dry_run {
                println!("{} | {} -> {}", name, phone, fixed);
            } else {
                println!("{} | {}", name, phone);
            }
        }

        if fix && !dry_run {
            let resource_name = person
                .resource_name
                .as_deref()
                .ok_or("Contact missing resource name")?;

            let mut updated = person.clone();
            if let Some(ref mut nums) = updated.phone_numbers {
                for pn in nums.iter_mut() {
                    if let Some(ref val) = pn.value {
                        if is_fixable_phone(val) && !has_country_code(val) {
                            pn.value = Some(add_country_code(val, country));
                        }
                    }
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                .doit()
                .await?;
            eprintln!("  Fixed: {}", name);
        }
    }

    Ok(())
}

async fn cmd_fix_phone_remove_whitespace(dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut contacts: Vec<google_people1::api::Person> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(&["names", "phoneNumbers", "metadata"]));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            for person in connections {
                let has_whitespace = person
                    .phone_numbers
                    .as_ref()
                    .is_some_and(|nums| nums.iter().any(|p| {
                        p.value.as_deref().is_some_and(|v| v.contains(char::is_whitespace))
                    }));

                if has_whitespace {
                    contacts.push(person);
                }
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    for person in &contacts {
        let name = person
            .names
            .as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.display_name.as_deref())
            .unwrap_or("<no name>");

        if let Some(nums) = &person.phone_numbers {
            for pn in nums {
                if let Some(val) = pn.value.as_deref() {
                    if val.contains(char::is_whitespace) {
                        let fixed: String = val.chars().filter(|c| !c.is_whitespace()).collect();
                        println!("{} | {} -> {}", name, val, fixed);
                    }
                }
            }
        }

        if !dry_run {
            let resource_name = person
                .resource_name
                .as_deref()
                .ok_or("Contact missing resource name")?;

            let mut updated = person.clone();
            if let Some(ref mut nums) = updated.phone_numbers {
                for pn in nums.iter_mut() {
                    if let Some(ref val) = pn.value {
                        if val.contains(char::is_whitespace) {
                            pn.value = Some(val.chars().filter(|c| !c.is_whitespace()).collect());
                        }
                    }
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                .doit()
                .await?;
            eprintln!("  Fixed: {}", name);
        }
    }

    Ok(())
}

async fn cmd_fix_remove_minus(dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut contacts: Vec<google_people1::api::Person> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(&["names", "phoneNumbers", "metadata"]));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            for person in connections {
                let has_minus = person
                    .phone_numbers
                    .as_ref()
                    .is_some_and(|nums| nums.iter().any(|p| {
                        p.value.as_deref().is_some_and(|v| v.contains('-'))
                    }));

                if has_minus {
                    contacts.push(person);
                }
            }
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    for person in &contacts {
        let name = person
            .names
            .as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.display_name.as_deref())
            .unwrap_or("<no name>");

        if let Some(nums) = &person.phone_numbers {
            for pn in nums {
                if let Some(val) = pn.value.as_deref() {
                    if val.contains('-') {
                        let fixed = val.replace('-', "");
                        println!("{} | {} -> {}", name, val, fixed);
                    }
                }
            }
        }

        if !dry_run {
            let resource_name = person
                .resource_name
                .as_deref()
                .ok_or("Contact missing resource name")?;

            let mut updated = person.clone();
            if let Some(ref mut nums) = updated.phone_numbers {
                for pn in nums.iter_mut() {
                    if let Some(ref val) = pn.value {
                        if val.contains('-') {
                            pn.value = Some(val.replace('-', ""));
                        }
                    }
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                .doit()
                .await?;
            eprintln!("  Fixed: {}", name);
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
        Commands::CheckEnglish { fix } => cmd_check_english(fix).await?,
        Commands::CheckCaps { fix } => cmd_check_caps(fix).await?,
        Commands::CheckPhone { fix, dry_run, ref country } => cmd_check_phone(fix, dry_run, country).await?,
        Commands::FixRemoveMinus { dry_run } => cmd_fix_remove_minus(dry_run).await?,
        Commands::FixPhoneRemoveWhitespace { dry_run } => cmd_fix_phone_remove_whitespace(dry_run).await?,
        Commands::Version => {
            let is_dirty = std::process::Command::new("git")
                .args(["diff", "--quiet", "HEAD"])
                .status()
                .is_ok_and(|s| !s.success());
            let dirty_str = if is_dirty { "true" } else { "false" };
            let describe = if is_dirty {
                format!("{}-dirty", env!("GIT_DESCRIBE"))
            } else {
                env!("GIT_DESCRIBE").to_string()
            };
            println!("rscontacts {} by {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"));
            println!("GIT_DESCRIBE: {}", describe);
            println!("GIT_SHA: {}", env!("GIT_SHA"));
            println!("GIT_BRANCH: {}", env!("GIT_BRANCH"));
            println!("GIT_DIRTY: {}", dirty_str);
            println!("RUSTC_SEMVER: {}", env!("RUSTC_SEMVER"));
            println!("RUST_EDITION: {}", env!("RUST_EDITION"));
        }
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
    fn test_cli_check_english_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-english"]);
        assert!(matches!(cli.command, Commands::CheckEnglish { fix: false }));
    }

    #[test]
    fn test_cli_check_english_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-english", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckEnglish { fix: true }));
    }

    #[test]
    fn test_cli_check_caps_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-caps"]);
        assert!(matches!(cli.command, Commands::CheckCaps { fix: false }));
    }

    #[test]
    fn test_cli_check_caps_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-caps", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckCaps { fix: true }));
    }

    #[test]
    fn test_is_all_caps_true() {
        assert!(is_all_caps("JOHN DOE"));
        assert!(is_all_caps("MARK VELTZER"));
        assert!(is_all_caps("JEAN-PIERRE"));
    }

    #[test]
    fn test_is_all_caps_false() {
        assert!(!is_all_caps("John Doe"));
        assert!(!is_all_caps("john doe"));
        assert!(!is_all_caps("JOHN doe"));
    }

    #[test]
    fn test_is_all_caps_no_alpha() {
        assert!(!is_all_caps("123"));
        assert!(!is_all_caps(""));
    }

    #[test]
    fn test_is_all_caps_short_codes() {
        assert!(!is_all_caps("P78"));
        assert!(!is_all_caps("A1"));
    }

    #[test]
    fn test_cli_check_phone_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-phone"]);
        assert!(matches!(cli.command, Commands::CheckPhone { fix: false, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_phone_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-phone", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckPhone { fix: true, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_phone_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "check-phone", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::CheckPhone { fix: true, dry_run: true, .. }));
    }

    #[test]
    fn test_cli_check_phone_custom_country() {
        let cli = Cli::parse_from(["rscontacts", "check-phone", "--country", "1"]);
        if let Commands::CheckPhone { country, .. } = cli.command {
            assert_eq!(country, "1");
        } else {
            panic!("wrong command");
        }
    }

    #[test]
    fn test_add_country_code_with_leading_zero() {
        assert_eq!(add_country_code("0501234567", "972"), "+972501234567");
    }

    #[test]
    fn test_add_country_code_without_leading_zero() {
        assert_eq!(add_country_code("501234567", "972"), "+972501234567");
    }

    #[test]
    fn test_add_country_code_us() {
        assert_eq!(add_country_code("5551234", "1"), "+15551234");
    }

    #[test]
    fn test_is_fixable_phone_digits() {
        assert!(is_fixable_phone("0501234567"));
        assert!(is_fixable_phone("+972-50-123-4567"));
        assert!(is_fixable_phone("(02) 555-1234"));
    }

    #[test]
    fn test_is_fixable_phone_alpha() {
        assert!(!is_fixable_phone("BHapoalim"));
        assert!(!is_fixable_phone("HOME"));
        assert!(!is_fixable_phone(""));
    }

    #[test]
    fn test_is_fixable_phone_mixed() {
        assert!(!is_fixable_phone("1800FLOWERS"));
        assert!(!is_fixable_phone("P78"));
    }

    #[test]
    fn test_is_fixable_phone_star() {
        assert!(!is_fixable_phone("*9848"));
        assert!(!is_fixable_phone("*100"));
    }

    #[test]
    fn test_is_fixable_phone_short() {
        assert!(!is_fixable_phone("100"));
        assert!(!is_fixable_phone("199"));
        assert!(!is_fixable_phone("911"));
    }

    #[test]
    fn test_has_country_code_plus() {
        assert!(has_country_code("+1-555-1234"));
        assert!(has_country_code("+972501234567"));
        assert!(has_country_code("  +44 20 7946 0958"));
    }

    #[test]
    fn test_has_country_code_double_zero() {
        assert!(has_country_code("00972501234567"));
        assert!(has_country_code("0044 20 7946 0958"));
    }

    #[test]
    fn test_has_country_code_missing() {
        assert!(!has_country_code("0501234567"));
        assert!(!has_country_code("555-1234"));
        assert!(!has_country_code("(02) 555-1234"));
    }

    #[test]
    fn test_has_country_code_empty() {
        assert!(!has_country_code(""));
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
