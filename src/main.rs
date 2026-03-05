use clap::{CommandFactory, Parser, Subcommand};
use google_people1::api::ListConnectionsResponse;
use google_people1::{FieldMask, PeopleService};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

const MUTATE_DELAY: Duration = Duration::from_millis(500);


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
        /// Force re-authentication even if a token is already cached
        #[arg(long)]
        force: bool,
    },
    /// List all contacts
    List {
        /// Also show email addresses
        #[arg(long)]
        emails: bool,
        /// Also show phone labels (type)
        #[arg(long)]
        labels: bool,
    },
    /// Print contacts with non-English names
    CheckNameEnglish {
        /// Interactively fix each non-English contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts with all-caps names
    CheckNameCaps {
        /// Interactively fix each all-caps contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts whose name doesn't start with a capital letter
    CheckNameFirstCapitalLetter {
        /// Interactively fix each contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts with reversed name order (e.g. "Family, Given")
    CheckNameOrder {
        /// Interactively fix each contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Run all checks
    CheckAll {
        /// Fix all issues found
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
        /// Only show error counts per check, no details
        #[arg(long)]
        stats: bool,
        /// Country code to prepend for phone country code check (without +)
        #[arg(long, default_value = "972")]
        country: String,
    },
    /// Print contacts with phone numbers missing a country code
    CheckPhoneCountrycode {
        /// Fix by prepending country code
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
        /// Country code to prepend (without +)
        #[arg(long, default_value = "972")]
        country: String,
    },
    /// Print phone numbers not in +CC-NUMBER format
    CheckPhoneFormat {
        /// Fix phone numbers to +CC-NUMBER format
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
        /// Country code to use when formatting
        #[arg(long, default_value = "972")]
        country: String,
    },
    /// Print contacts not assigned to any label (contact group)
    CheckContactNoLabel,
    /// Print contacts with phone numbers missing a label (mobile/home/work/etc)
    CheckPhoneNoLabel,
    /// Print contacts with non-English phone labels
    CheckPhoneLabelEnglish {
        /// Interactively fix non-English phone labels
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts with invalid-looking email addresses
    CheckEmail,
    /// Print contacts with uppercase letters in email addresses
    CheckEmailCaps {
        /// Automatically lowercase emails
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts that have the same email address attached twice
    CheckDuplicateEmails {
        /// Interactively remove duplicate email addresses
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts that have the same phone number attached twice
    CheckDuplicatePhones {
        /// Interactively remove duplicate phone numbers
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print labels (contact groups) that have no contacts
    CheckLabelsNophone {
        /// Delete empty labels
        #[arg(long)]
        fix: bool,
        /// Show what would be deleted without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contact labels (groups) that have a space in their name
    CheckContactLabelSpace {
        /// Interactively rename labels with spaces
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Show all distinct phone labels in use
    ShowPhoneLabels,
    /// Show all contact labels (contact groups) in use
    ShowContactLabels,
    /// Show all details about a specific contact
    ShowContact {
        /// Name (or part of name) to search for
        name: String,
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

fn is_valid_email(email: &str) -> bool {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return false;
    }
    let Some((local, domain)) = trimmed.rsplit_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    let tld = domain.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 {
        return false;
    }
    true
}

fn phone_has_type(pn: &google_people1::api::PhoneNumber) -> bool {
    !pn.type_.as_deref().unwrap_or("").is_empty()
        || !pn.formatted_type.as_deref().unwrap_or("").is_empty()
}

fn starts_with_capital(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
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

fn is_correct_phone_format(phone: &str) -> bool {
    let trimmed = phone.trim();
    if !trimmed.starts_with('+') {
        return false;
    }
    let rest = &trimmed[1..];
    let Some(dash_pos) = rest.find('-') else {
        return false;
    };
    let cc = &rest[..dash_pos];
    let number = &rest[dash_pos + 1..];
    !cc.is_empty()
        && cc.chars().all(|c| c.is_ascii_digit())
        && COUNTRY_CODES.contains(&cc)
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
        && !rest[dash_pos + 1..].contains('-')
}

const COUNTRY_CODES: &[&str] = &[
    "1", "7",
    "20", "27", "30", "31", "32", "33", "34", "36", "39",
    "40", "41", "43", "44", "45", "46", "47", "48", "49",
    "51", "52", "53", "54", "55", "56", "57", "58",
    "60", "61", "62", "63", "64", "65", "66",
    "81", "82", "84", "86", "90", "91", "92", "93", "94", "95", "98",
    "212", "213", "216", "218",
    "220", "221", "222", "223", "224", "225", "226", "229",
    "230", "231", "232", "233", "234", "235", "236", "238", "239",
    "240", "241", "242", "243", "244", "245", "246", "247", "248", "249",
    "250", "251", "252", "253", "254", "255", "256", "257", "258",
    "260", "262", "263", "264", "265", "266", "267", "268", "269",
    "290", "297", "298", "299",
    "350", "351", "352", "353", "354", "355", "356", "357", "358", "359",
    "370", "371", "372", "373", "374", "375", "376", "377", "378", "379",
    "380", "381", "382", "385", "386", "387", "388", "389",
    "420", "423",
    "500", "501", "502", "503", "504", "505", "506", "507", "508", "509",
    "590", "591", "592", "593", "594", "595", "596", "597", "598", "599",
    "672", "673", "674", "675", "676", "677", "678", "679",
    "680", "681", "682", "683", "684", "685", "686", "687", "688", "689",
    "690", "691", "692",
    "800", "808", "850", "852", "853", "855", "856",
    "870", "878", "880", "881", "882", "883", "886", "888",
    "891", "900",
    "960", "961", "962", "963", "964", "965", "966", "967", "968",
    "970", "971", "972", "973", "974", "975", "976", "977", "979",
    "992", "993", "994", "995", "996", "998",
];

fn detect_country_code(digits: &str) -> Option<usize> {
    // Try longest match first (3 digits, then 2, then 1)
    for len in (1..=3).rev() {
        if digits.len() > len {
            let prefix = &digits[..len];
            if COUNTRY_CODES.contains(&prefix) {
                return Some(len);
            }
        }
    }
    None
}

fn fix_phone_format(phone: &str, country: &str) -> String {
    let trimmed = phone.trim();
    let has_plus = trimmed.starts_with('+');
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    if has_plus || trimmed.starts_with("00") {
        // Already has country code — extract it
        let pure_digits = if trimmed.starts_with("00") {
            &digits[2..]
        } else {
            &digits[..]
        };
        if let Some(cc_len) = detect_country_code(pure_digits) {
            format!("+{}-{}", &pure_digits[..cc_len], &pure_digits[cc_len..])
        } else {
            // Unknown country code, use default
            format!("+{}-{}", country, pure_digits)
        }
    } else {
        // No country code — add it, strip leading 0
        let without_zero = digits.strip_prefix('0').unwrap_or(&digits);
        format!("+{}-{}", country, without_zero)
    }
}

// --- Helper functions ---

type HubType = PeopleService<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

async fn fetch_all_contacts(hub: &HubType, fields: &[&str]) -> Result<Vec<google_people1::api::Person>, Box<dyn std::error::Error>> {
    let mut all: Vec<google_people1::api::Person> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .people()
            .connections_list("people/me")
            .person_fields(FieldMask::new::<&str>(fields));

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_response, result): (_, ListConnectionsResponse) = request.doit().await?;

        if let Some(connections) = result.connections {
            all.extend(connections);
        }

        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(all)
}

fn person_name(person: &google_people1::api::Person) -> &str {
    person
        .names
        .as_ref()
        .and_then(|names| names.first())
        .and_then(|n| n.display_name.as_deref())
        .unwrap_or("")
}

fn person_display_name(person: &google_people1::api::Person) -> &str {
    let name = person_name(person);
    if name.is_empty() { "<no name>" } else { name }
}

fn person_email(person: &google_people1::api::Person) -> &str {
    person
        .email_addresses
        .as_ref()
        .and_then(|emails| emails.first())
        .and_then(|e| e.value.as_deref())
        .unwrap_or("")
}

async fn interactive_name_fix(hub: &HubType, person: &google_people1::api::Person, name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'd' => {
            hub.people().delete_contact(resource_name).doit().await?;
            eprintln!("  Deleted.");
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        's' => {
            eprintln!("  Skipped.");
        }
        _ => unreachable!(),
    }
    Ok(())
}

async fn update_phone_numbers<F>(hub: &HubType, person: &google_people1::api::Person, transform: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Option<String>,
{
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    let mut updated = person.clone();
    if let Some(ref mut nums) = updated.phone_numbers {
        for pn in nums.iter_mut() {
            if let Some(ref val) = pn.value {
                if let Some(new_val) = transform(val) {
                    pn.value = Some(new_val);
                }
            }
        }
    }
    hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
        .doit()
        .await?;
    eprintln!("  Fixed: {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

// --- End helper functions ---

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

async fn build_hub() -> Result<HubType, Box<dyn std::error::Error>> {
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

async fn cmd_auth(no_browser: bool, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    if force {
        let cache = token_cache_path();
        if cache.exists() {
            std::fs::remove_file(&cache)?;
            eprintln!("Removed cached token at {}", cache.display());
        }
    }
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

async fn cmd_list(emails: bool, labels: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let mut fields = vec!["names", "phoneNumbers"];
    if emails { fields.push("emailAddresses"); }
    let contacts = fetch_all_contacts(&hub, &fields).await?;

    for person in &contacts {
        let name = person_display_name(person);

        if labels {
            if let Some(nums) = &person.phone_numbers {
                for pn in nums {
                    let phone = pn.value.as_deref().unwrap_or("");
                    let label = pn.type_.as_deref()
                        .or(pn.formatted_type.as_deref())
                        .unwrap_or("<no label>");
                    if emails {
                        println!("{} | {} | {} [{}]", name, person_email(person), phone, label);
                    } else {
                        println!("{} | {} [{}]", name, phone, label);
                    }
                }
            } else if emails {
                println!("{} | {}", name, person_email(person));
            } else {
                println!("{}", name);
            }
        } else {
            let phone = person
                .phone_numbers
                .as_ref()
                .and_then(|phones| phones.first())
                .and_then(|p| p.value.as_deref())
                .unwrap_or("");

            if emails {
                let email = person_email(person);
                if !email.is_empty() || !phone.is_empty() {
                    println!("{} | {} | {}", name, email, phone);
                } else {
                    println!("{}", name);
                }
            } else if !phone.is_empty() {
                println!("{} | {}", name, phone);
            } else {
                println!("{}", name);
            }
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

fn print_name_with_email(name: &str, email: &str, prefix: &str) {
    if !email.is_empty() {
        println!("{}{} | {}", prefix, name, email);
    } else {
        println!("{}{}", prefix, name);
    }
}

async fn check_name_issues<F>(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    predicate: F,
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
) -> Result<usize, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> bool,
{
    let filtered: Vec<&google_people1::api::Person> = contacts.iter().filter(|p| {
        let name = person_name(p);
        !name.is_empty() && predicate(name)
    }).collect();

    if !filtered.is_empty() {
        if let Some(header) = header {
            println!("=== {} ({}) ===", header, filtered.len());
        }
    }

    for person in &filtered {
        let name = person_name(person);
        let email = person_email(person);
        print_name_with_email(name, email, prefix);

        if fix && !dry_run {
            interactive_name_fix(hub, person, name).await?;
        }
    }

    if !filtered.is_empty() && header.is_some() {
        println!();
    }

    Ok(filtered.len())
}

async fn check_phone_issues<P, T>(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    predicate: P,
    transform: T,
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
) -> Result<usize, Box<dyn std::error::Error>>
where
    P: Fn(&str) -> bool,
    T: Fn(&str) -> String + Clone,
{
    let filtered: Vec<&google_people1::api::Person> = contacts.iter().filter(|p| {
        p.phone_numbers.as_ref().is_some_and(|nums| nums.iter().any(|pn| {
            pn.value.as_deref().is_some_and(|v| predicate(v))
        }))
    }).collect();

    if !filtered.is_empty() {
        if let Some(header) = header {
            println!("=== {} ({}) ===", header, filtered.len());
        }
    }

    for person in &filtered {
        let name = person_display_name(person);

        if let Some(nums) = &person.phone_numbers {
            for pn in nums {
                if let Some(val) = pn.value.as_deref() {
                    if predicate(val) {
                        let fixed = transform(val);
                        print_phone_fix(name, val, &fixed, fix, dry_run, prefix);
                    }
                }
            }
        }

        if fix && !dry_run {
            let transform = transform.clone();
            update_phone_numbers(hub, person, |val| {
                if predicate(val) {
                    Some(transform(val))
                } else {
                    None
                }
            }).await?;
        }
    }

    if !filtered.is_empty() && header.is_some() {
        println!();
    }

    Ok(filtered.len())
}

async fn cmd_check_english(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "metadata"]).await?;
    check_name_issues(&hub, &contacts, |name| !is_english_name(name), fix, dry_run, "", None).await?;
    Ok(())
}

async fn cmd_check_caps(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "metadata"]).await?;
    check_name_issues(&hub, &contacts, |name| is_all_caps(name), fix, dry_run, "", None).await?;
    Ok(())
}

async fn cmd_check_first_capital_letter(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "metadata"]).await?;
    check_name_issues(&hub, &contacts, |name| !starts_with_capital(name), fix, dry_run, "", None).await?;
    Ok(())
}

fn has_reversed_name(person: &google_people1::api::Person) -> bool {
    person_name(person).contains(',')
}

fn compute_fixed_name(display_name: &str) -> String {
    if let Some((family, given)) = display_name.split_once(',') {
        let given = given.trim();
        let family = family.trim();
        if !given.is_empty() && !family.is_empty() {
            return format!("{} {}", given, family);
        }
    }
    display_name.to_string()
}

fn check_name_order(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if has_reversed_name(person) {
            if count == 0 {
                if let Some(header) = header {
                    println!("=== {} ===", header);
                }
            }
            let name = person_display_name(person);
            let fixed = compute_fixed_name(name);
            println!("{}{} -> {}", prefix, name, fixed);
            count += 1;
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

async fn cmd_check_name_order(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "metadata"]).await?;

    for person in &contacts {
        if has_reversed_name(person) {
            let name = person_display_name(person);
            let fixed = compute_fixed_name(name);
            println!("{} -> {}", name, fixed);

            if fix && !dry_run {
                use std::io::Write;
                std::io::stdout().flush()?;
                let resource_name = person
                    .resource_name
                    .as_deref()
                    .ok_or("Contact missing resource name")?;

                match prompt_fix_action(name)? {
                    'r' => {
                        let new_name = prompt_new_name_with_default(name, &fixed)?;
                        let mut updated = person.clone();
                        if let Some(ref mut names) = updated.names {
                            if let Some(first) = names.first_mut() {
                                if let Some((given, family)) = new_name.split_once(' ') {
                                    first.given_name = Some(given.to_string());
                                    first.family_name = Some(family.to_string());
                                } else {
                                    first.given_name = Some(new_name.clone());
                                    first.family_name = None;
                                }
                                first.unstructured_name = Some(new_name.clone());
                            }
                        }
                        hub.people()
                            .update_contact(updated, resource_name)
                            .update_person_fields(FieldMask::new::<&str>(&["names"]))
                            .doit()
                            .await?;
                        eprintln!("  Renamed to \"{}\"", new_name);
                        tokio::time::sleep(MUTATE_DELAY).await;
                    }
                    'd' => {
                        hub.people().delete_contact(resource_name).doit().await?;
                        eprintln!("  Deleted.");
                        tokio::time::sleep(MUTATE_DELAY).await;
                    }
                    's' => {
                        eprintln!("  Skipped.");
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    Ok(())
}

fn prompt_new_name_with_default(old_name: &str, suggested: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Write;
    loop {
        eprint!("  New name for \"{}\" [Enter for \"{}\"]: ", old_name, suggested);
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(suggested.to_string());
        }
        return Ok(trimmed.to_string());
    }
}

fn print_phone_fix(name: &str, phone: &str, fixed: &str, fix: bool, dry_run: bool, prefix: &str) {
    if fix || dry_run {
        println!("{}{} | {} -> {}", prefix, name, phone, fixed);
    } else {
        println!("{}{} | {}", prefix, name, phone);
    }
}

async fn cmd_check_phone_countrycode(fix: bool, dry_run: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers", "metadata"]).await?;
    let country = country.to_string();
    check_phone_issues(
        &hub, &contacts,
        |v| is_fixable_phone(v) && !has_country_code(v),
        move |v| add_country_code(v, &country),
        fix, dry_run, "", None,
    ).await?;
    Ok(())
}

async fn cmd_check_phone_format(fix: bool, dry_run: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers", "metadata"]).await?;
    let country_owned = country.to_string();
    check_phone_issues(
        &hub, &contacts,
        |v| is_fixable_phone(v) && !is_correct_phone_format(v),
        move |v| fix_phone_format(v, &country_owned),
        fix, dry_run, "", None,
    ).await?;
    Ok(())
}

fn check_duplicate_phones(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if let Some(nums) = &person.phone_numbers {
            let values: Vec<&str> = nums.iter().filter_map(|pn| pn.value.as_deref()).collect();
            let mut seen = std::collections::HashSet::new();
            let dupes: Vec<&str> = values.iter().filter(|v| !seen.insert(**v)).copied().collect();
            if !dupes.is_empty() {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let name = person_display_name(person);
                for phone in &dupes {
                    println!("{}{} | {}", prefix, name, phone);
                    count += 1;
                }
            }
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

fn check_invalid_emails(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if let Some(emails) = &person.email_addresses {
            for email in emails {
                if let Some(val) = email.value.as_deref() {
                    if !is_valid_email(val) {
                        if count == 0 {
                            if let Some(header) = header {
                                println!("=== {} ===", header);
                            }
                        }
                        let name = person_display_name(person);
                        println!("{}{} | {}", prefix, name, val);
                        count += 1;
                    }
                }
            }
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

fn has_user_label(person: &google_people1::api::Person) -> bool {
    person.memberships.as_ref().is_some_and(|memberships| {
        memberships.iter().any(|m| {
            m.contact_group_membership.as_ref().is_some_and(|cgm| {
                let rn = cgm.contact_group_resource_name.as_deref().unwrap_or("");
                !rn.is_empty() && rn != "contactGroups/myContacts"
            })
        })
    })
}

fn check_no_label(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if !has_user_label(person) {
            if count == 0 {
                if let Some(header) = header {
                    println!("=== {} ===", header);
                }
            }
            let name = person_display_name(person);
            let email = person_email(person);
            print_name_with_email(name, email, prefix);
            count += 1;
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

fn prompt_remove_duplicate(name: &str, phone: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use std::io::Write;
    loop {
        eprint!("  Remove duplicate \"{}\" from {}? [y/n] ", phone, name);
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim().chars().next() {
            Some('y') => return Ok(true),
            Some('n') => return Ok(false),
            _ => eprintln!("  Invalid choice. Enter y or n."),
        }
    }
}

async fn remove_duplicate_phones(hub: &HubType, person: &google_people1::api::Person) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    let mut updated = person.clone();
    if let Some(ref mut nums) = updated.phone_numbers {
        let mut seen = std::collections::HashSet::new();
        nums.retain(|pn| {
            let val = pn.value.as_deref().unwrap_or("");
            seen.insert(val.to_string())
        });
    }
    hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
        .doit()
        .await?;
    eprintln!("  Removed duplicates for {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

async fn cmd_check_duplicate_phones(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers", "metadata"]).await?;

    for person in &contacts {
        if let Some(nums) = &person.phone_numbers {
            let values: Vec<&str> = nums.iter().filter_map(|pn| pn.value.as_deref()).collect();
            let mut seen = std::collections::HashSet::new();
            let dupes: Vec<&str> = values.iter().filter(|v| !seen.insert(**v)).copied().collect();
            if !dupes.is_empty() {
                let name = person_display_name(person);
                for phone in &dupes {
                    println!("{} | {}", name, phone);
                }
                if fix && !dry_run {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let name = person_display_name(person);
                    if prompt_remove_duplicate(name, &dupes.join(", "))? {
                        remove_duplicate_phones(&hub, person).await?;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
        }
    }

    Ok(())
}

async fn cmd_check_email() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "metadata"]).await?;
    check_invalid_emails(&contacts, "", None);
    Ok(())
}

fn check_email_caps(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if let Some(emails) = &person.email_addresses {
            for email in emails {
                if let Some(val) = email.value.as_deref() {
                    if val != val.to_lowercase().as_str() {
                        if count == 0 {
                            if let Some(header) = header {
                                println!("=== {} ===", header);
                            }
                        }
                        let name = person_display_name(person);
                        println!("{}{} | {} -> {}", prefix, name, val, val.to_lowercase());
                        count += 1;
                    }
                }
            }
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

async fn cmd_check_email_caps(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "metadata"]).await?;

    for person in &contacts {
        if let Some(emails) = &person.email_addresses {
            let has_caps = emails.iter().any(|e| {
                e.value.as_deref().is_some_and(|v| v != v.to_lowercase().as_str())
            });
            if has_caps {
                let name = person_display_name(person);
                for email in emails {
                    if let Some(val) = email.value.as_deref() {
                        if val != val.to_lowercase().as_str() {
                            println!("{} | {} -> {}", name, val, val.to_lowercase());
                        }
                    }
                }
                if fix && !dry_run {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let resource_name = person
                        .resource_name
                        .as_deref()
                        .ok_or("Contact missing resource name")?;
                    let mut updated = person.clone();
                    if let Some(ref mut ems) = updated.email_addresses {
                        for e in ems.iter_mut() {
                            if let Some(ref val) = e.value {
                                e.value = Some(val.to_lowercase());
                            }
                        }
                    }
                    hub.people()
                        .update_contact(updated, resource_name)
                        .update_person_fields(FieldMask::new::<&str>(&["emailAddresses"]))
                        .doit()
                        .await?;
                    eprintln!("  Fixed emails for {}", name);
                    tokio::time::sleep(MUTATE_DELAY).await;
                }
            }
        }
    }

    Ok(())
}

fn check_duplicate_emails(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if let Some(emails) = &person.email_addresses {
            let values: Vec<&str> = emails.iter().filter_map(|e| e.value.as_deref()).collect();
            let mut seen = std::collections::HashSet::new();
            let dupes: Vec<&str> = values.iter().filter(|v| !seen.insert(**v)).copied().collect();
            if !dupes.is_empty() {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let name = person_display_name(person);
                for email in &dupes {
                    println!("{}{} | {}", prefix, name, email);
                    count += 1;
                }
            }
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

async fn remove_duplicate_emails(hub: &HubType, person: &google_people1::api::Person) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    let mut updated = person.clone();
    if let Some(ref mut emails) = updated.email_addresses {
        let mut seen = std::collections::HashSet::new();
        emails.retain(|e| {
            let val = e.value.as_deref().unwrap_or("");
            seen.insert(val.to_string())
        });
    }
    hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["emailAddresses"]))
        .doit()
        .await?;
    eprintln!("  Removed duplicates for {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

async fn cmd_check_duplicate_emails(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "metadata"]).await?;

    for person in &contacts {
        if let Some(emails) = &person.email_addresses {
            let values: Vec<&str> = emails.iter().filter_map(|e| e.value.as_deref()).collect();
            let mut seen = std::collections::HashSet::new();
            let dupes: Vec<&str> = values.iter().filter(|v| !seen.insert(**v)).copied().collect();
            if !dupes.is_empty() {
                let name = person_display_name(person);
                for email in &dupes {
                    println!("{} | {}", name, email);
                }
                if fix && !dry_run {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let name = person_display_name(person);
                    if prompt_remove_duplicate(name, &dupes.join(", "))? {
                        remove_duplicate_emails(&hub, person).await?;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
        }
    }

    Ok(())
}

fn check_phone_no_label(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if let Some(nums) = &person.phone_numbers {
            let untyped: Vec<&str> = nums.iter()
                .filter(|pn| !phone_has_type(pn))
                .filter_map(|pn| pn.value.as_deref())
                .collect();
            if !untyped.is_empty() {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let name = person_display_name(person);
                for phone in &untyped {
                    println!("{}{} | {}", prefix, name, phone);
                    count += 1;
                }
            }
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

async fn cmd_check_phone_no_label() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers", "metadata"]).await?;
    check_phone_no_label(&contacts, "", None);
    Ok(())
}

fn check_phone_label_english(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>) -> usize {
    let mut count = 0;
    for person in contacts {
        if let Some(nums) = &person.phone_numbers {
            for pn in nums {
                let label = pn.formatted_type.as_deref()
                    .or(pn.type_.as_deref())
                    .unwrap_or("");
                if !label.is_empty() && !label.chars().all(|c| c.is_ascii()) {
                    if count == 0 {
                        if let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    }
                    let name = person_display_name(person);
                    let phone = pn.value.as_deref().unwrap_or("");
                    println!("{}{} | {} [{}]", prefix, name, phone, label);
                    count += 1;
                }
            }
        }
    }
    if count > 0 && header.is_some() { println!(); }
    count
}

async fn cmd_check_phone_label_english(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers", "metadata"]).await?;

    for person in &contacts {
        if let Some(nums) = &person.phone_numbers {
            let has_non_english = nums.iter().any(|pn| {
                let label = pn.formatted_type.as_deref()
                    .or(pn.type_.as_deref())
                    .unwrap_or("");
                !label.is_empty() && !label.chars().all(|c| c.is_ascii())
            });
            if has_non_english {
                let name = person_display_name(person);
                for pn in nums {
                    let label = pn.formatted_type.as_deref()
                        .or(pn.type_.as_deref())
                        .unwrap_or("");
                    if !label.is_empty() && !label.chars().all(|c| c.is_ascii()) {
                        let phone = pn.value.as_deref().unwrap_or("");
                        println!("{} | {} [{}]", name, phone, label);
                    }
                }
                if fix && !dry_run {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let new_label = prompt_phone_label_fix(name)?;
                    if let Some(new_label) = new_label {
                        fix_phone_labels_english(&hub, person, &new_label).await?;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
        }
    }

    Ok(())
}

fn prompt_phone_label_fix(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    use std::io::Write;
    let options = ["mobile", "home", "work", "main", "other"];
    loop {
        eprint!("  Label for {}'s phone? [m]obile/[h]ome/[w]ork/m[a]in/[o]ther/[s]kip: ", name);
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim().chars().next() {
            Some('m') => return Ok(Some(options[0].to_string())),
            Some('h') => return Ok(Some(options[1].to_string())),
            Some('w') => return Ok(Some(options[2].to_string())),
            Some('a') => return Ok(Some(options[3].to_string())),
            Some('o') => return Ok(Some(options[4].to_string())),
            Some('s') => return Ok(None),
            _ => eprintln!("  Invalid choice. Enter m, h, w, a, o, or s."),
        }
    }
}

async fn fix_phone_labels_english(hub: &HubType, person: &google_people1::api::Person, new_label: &str) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    let mut updated = person.clone();
    if let Some(ref mut nums) = updated.phone_numbers {
        for pn in nums.iter_mut() {
            let label = pn.formatted_type.as_deref()
                .or(pn.type_.as_deref())
                .unwrap_or("");
            if !label.is_empty() && !label.chars().all(|c| c.is_ascii()) {
                pn.type_ = Some(new_label.to_string());
                pn.formatted_type = Some(new_label.to_string());
            }
        }
    }
    hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
        .doit()
        .await?;
    eprintln!("  Fixed labels for {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

async fn cmd_check_contact_no_label() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "memberships", "metadata"]).await?;
    check_no_label(&contacts, "", None);
    Ok(())
}

fn prompt_delete_label(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use std::io::Write;
    loop {
        eprint!("  Delete label \"{}\"? [y/n] ", name);
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim().chars().next() {
            Some('y') => return Ok(true),
            Some('n') => return Ok(false),
            _ => eprintln!("  Invalid choice. Enter y or n."),
        }
    }
}

async fn cmd_check_labels_nophone(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;

    let mut all_groups: Vec<google_people1::api::ContactGroup> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub.contact_groups().list();
        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }
        let (_response, result) = request.doit().await?;
        if let Some(groups) = result.contact_groups {
            all_groups.extend(groups);
        }
        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    let empty_groups: Vec<&google_people1::api::ContactGroup> = all_groups.iter().filter(|g| {
        let count = g.member_count.unwrap_or(0);
        let is_user_group = g.group_type.as_deref() == Some("USER_CONTACT_GROUP");
        count == 0 && is_user_group
    }).collect();

    for group in &empty_groups {
        let name = group.name.as_deref().unwrap_or("<unnamed>");
        println!("{}", name);

        if fix && !dry_run {
            use std::io::Write;
            std::io::stdout().flush()?;
            if let Some(resource_name) = group.resource_name.as_deref() {
                if prompt_delete_label(name)? {
                    hub.contact_groups().delete(resource_name).doit().await?;
                    eprintln!("  Deleted.");
                    tokio::time::sleep(MUTATE_DELAY).await;
                } else {
                    eprintln!("  Skipped.");
                }
            }
        }
    }

    Ok(())
}

async fn cmd_show_contact(search: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_fields = &[
        "names", "emailAddresses", "phoneNumbers", "addresses", "birthdays",
        "organizations", "memberships", "biographies", "urls", "events",
        "relations", "nicknames", "occupations", "interests", "skills",
        "userDefined", "imClients", "sipAddresses", "locations",
        "externalIds", "clientData", "metadata",
    ];
    let contacts = fetch_all_contacts(&hub, all_fields).await?;
    let search_lower = search.to_lowercase();
    let matches: Vec<_> = contacts.iter().filter(|p| {
        person_name(p).to_lowercase().contains(&search_lower)
    }).collect();

    if matches.is_empty() {
        println!("No contacts found matching \"{}\"", search);
        return Ok(());
    }

    for (i, person) in matches.iter().enumerate() {
        if i > 0 { println!("\n{}", "=".repeat(60)); }
        print_person_details(person);
    }
    Ok(())
}

fn print_person_details(person: &google_people1::api::Person) {
    let name = person_display_name(person);
    println!("Name: {}", name);

    if let Some(names) = &person.names {
        for n in names {
            if let Some(given) = &n.given_name { println!("  Given name: {}", given); }
            if let Some(family) = &n.family_name { println!("  Family name: {}", family); }
            if let Some(middle) = &n.middle_name { println!("  Middle name: {}", middle); }
            if let Some(prefix) = &n.honorific_prefix { println!("  Prefix: {}", prefix); }
            if let Some(suffix) = &n.honorific_suffix { println!("  Suffix: {}", suffix); }
        }
    }

    if let Some(nicknames) = &person.nicknames {
        for n in nicknames {
            if let Some(val) = &n.value { println!("Nickname: {}", val); }
        }
    }

    if let Some(emails) = &person.email_addresses {
        for e in emails {
            let val = e.value.as_deref().unwrap_or("");
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("Email: {}", val); }
            else { println!("Email: {} [{}]", val, t); }
        }
    }

    if let Some(phones) = &person.phone_numbers {
        for p in phones {
            let val = p.value.as_deref().unwrap_or("");
            let t = p.formatted_type.as_deref().or(p.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("Phone: {}", val); }
            else { println!("Phone: {} [{}]", val, t); }
        }
    }

    if let Some(addrs) = &person.addresses {
        for a in addrs {
            let t = a.formatted_type.as_deref().or(a.type_.as_deref()).unwrap_or("");
            if let Some(formatted) = &a.formatted_value {
                if t.is_empty() { println!("Address: {}", formatted); }
                else { println!("Address [{}]: {}", t, formatted); }
            }
        }
    }

    if let Some(orgs) = &person.organizations {
        for o in orgs {
            let org_name = o.name.as_deref().unwrap_or("");
            let title = o.title.as_deref().unwrap_or("");
            let dept = o.department.as_deref().unwrap_or("");
            let mut parts = Vec::new();
            if !title.is_empty() { parts.push(title.to_string()); }
            if !org_name.is_empty() { parts.push(org_name.to_string()); }
            if !dept.is_empty() { parts.push(format!("({})", dept)); }
            if !parts.is_empty() { println!("Organization: {}", parts.join(", ")); }
        }
    }

    if let Some(bdays) = &person.birthdays {
        for b in bdays {
            if let Some(date) = &b.date {
                let y = date.year.unwrap_or(0);
                let m = date.month.unwrap_or(0);
                let d = date.day.unwrap_or(0);
                if y > 0 { println!("Birthday: {}-{:02}-{:02}", y, m, d); }
                else { println!("Birthday: {:02}-{:02}", m, d); }
            }
            if let Some(text) = &b.text { println!("Birthday: {}", text); }
        }
    }

    if let Some(relations) = &person.relations {
        for r in relations {
            let val = r.person.as_deref().unwrap_or("");
            let t = r.formatted_type.as_deref().or(r.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("Relation: {}", val); }
            else { println!("Relation: {} [{}]", val, t); }
        }
    }

    if let Some(events) = &person.events {
        for e in events {
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("event");
            if let Some(date) = &e.date {
                let y = date.year.unwrap_or(0);
                let m = date.month.unwrap_or(0);
                let d = date.day.unwrap_or(0);
                if y > 0 { println!("Event [{}]: {}-{:02}-{:02}", t, y, m, d); }
                else { println!("Event [{}]: {:02}-{:02}", t, m, d); }
            }
        }
    }

    if let Some(bios) = &person.biographies {
        for b in bios {
            if let Some(val) = &b.value { println!("Biography: {}", val); }
        }
    }

    if let Some(urls) = &person.urls {
        for u in urls {
            let val = u.value.as_deref().unwrap_or("");
            let t = u.formatted_type.as_deref().or(u.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("URL: {}", val); }
            else { println!("URL: {} [{}]", val, t); }
        }
    }

    if let Some(ims) = &person.im_clients {
        for im in ims {
            let val = im.username.as_deref().unwrap_or("");
            let proto = im.formatted_protocol.as_deref().or(im.protocol.as_deref()).unwrap_or("");
            if proto.is_empty() { println!("IM: {}", val); }
            else { println!("IM: {} [{}]", val, proto); }
        }
    }

    if let Some(sips) = &person.sip_addresses {
        for s in sips {
            if let Some(val) = &s.value { println!("SIP: {}", val); }
        }
    }

    if let Some(occupations) = &person.occupations {
        for o in occupations {
            if let Some(val) = &o.value { println!("Occupation: {}", val); }
        }
    }

    if let Some(interests) = &person.interests {
        for i in interests {
            if let Some(val) = &i.value { println!("Interest: {}", val); }
        }
    }

    if let Some(skills) = &person.skills {
        for s in skills {
            if let Some(val) = &s.value { println!("Skill: {}", val); }
        }
    }

    if let Some(locations) = &person.locations {
        for l in locations {
            if let Some(val) = &l.value { println!("Location: {}", val); }
        }
    }

    if let Some(ext_ids) = &person.external_ids {
        for e in ext_ids {
            let val = e.value.as_deref().unwrap_or("");
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("External ID: {}", val); }
            else { println!("External ID: {} [{}]", val, t); }
        }
    }

    if let Some(user_defined) = &person.user_defined {
        for u in user_defined {
            let key = u.key.as_deref().unwrap_or("");
            let val = u.value.as_deref().unwrap_or("");
            println!("Custom: {} = {}", key, val);
        }
    }

    if let Some(client_data) = &person.client_data {
        for c in client_data {
            let key = c.key.as_deref().unwrap_or("");
            let val = c.value.as_deref().unwrap_or("");
            println!("Client data: {} = {}", key, val);
        }
    }

    if let Some(memberships) = &person.memberships {
        let labels: Vec<&str> = memberships.iter().filter_map(|m| {
            m.contact_group_membership.as_ref().and_then(|cgm| {
                cgm.contact_group_resource_name.as_deref()
            })
        }).filter(|rn| *rn != "contactGroups/myContacts").collect();
        if !labels.is_empty() {
            println!("Labels: {}", labels.join(", "));
        }
    }

    if let Some(rn) = &person.resource_name {
        println!("Resource: {}", rn);
    }
}

async fn cmd_check_contact_label_space(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let mut all_groups: Vec<google_people1::api::ContactGroup> = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut request = hub.contact_groups().list();
        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }
        let (_response, result) = request.doit().await?;
        if let Some(groups) = result.contact_groups {
            all_groups.extend(groups);
        }
        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    let with_space: Vec<&google_people1::api::ContactGroup> = all_groups.iter().filter(|g| {
        g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
            && g.name.as_deref().unwrap_or("").contains(' ')
    }).collect();

    for group in &with_space {
        let name = group.name.as_deref().unwrap_or("<unnamed>");
        println!("{}", name);

        if fix && !dry_run {
            use std::io::Write;
            std::io::stdout().flush()?;
            if let Some(new_name) = prompt_rename_label(name)? {
                let resource_name = group.resource_name.as_deref()
                    .ok_or("Contact group missing resource name")?;
                let mut updated_group = (*group).clone();
                updated_group.name = Some(new_name.clone());
                let req = google_people1::api::UpdateContactGroupRequest {
                    contact_group: Some(updated_group),
                    read_group_fields: None,
                    update_group_fields: None,
                };
                hub.contact_groups().update(req, resource_name).doit().await?;
                eprintln!("  Renamed \"{}\" -> \"{}\"", name, new_name);
                tokio::time::sleep(MUTATE_DELAY).await;
            } else {
                eprintln!("  Skipped.");
            }
        }
    }

    Ok(())
}

fn prompt_rename_label(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    use std::io::Write;
    loop {
        eprint!("  New name for \"{}\" (or [s]kip): ", name);
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("s") {
            return Ok(None);
        }
        if trimmed.is_empty() {
            eprintln!("  Name cannot be empty.");
            continue;
        }
        if trimmed.contains(' ') {
            eprintln!("  Name must not contain spaces.");
            continue;
        }
        return Ok(Some(trimmed.to_string()));
    }
}

async fn cmd_show_phone_labels() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["phoneNumbers"]).await?;
    let mut labels = std::collections::BTreeSet::new();
    for person in &contacts {
        if let Some(nums) = &person.phone_numbers {
            for pn in nums {
                if let Some(label) = pn.formatted_type.as_deref().or(pn.type_.as_deref()) {
                    if !label.is_empty() {
                        labels.insert(label.to_string());
                    }
                }
            }
        }
    }
    for label in &labels {
        println!("{}", label);
    }
    Ok(())
}

async fn cmd_show_contact_labels() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let mut all_groups: Vec<google_people1::api::ContactGroup> = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut request = hub.contact_groups().list();
        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }
        let (_response, result) = request.doit().await?;
        if let Some(groups) = result.contact_groups {
            all_groups.extend(groups);
        }
        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }
    for group in &all_groups {
        if group.group_type.as_deref() == Some("USER_CONTACT_GROUP") {
            let name = group.name.as_deref().unwrap_or("<unnamed>");
            let count = group.member_count.unwrap_or(0);
            println!("{} ({})", name, count);
        }
    }
    Ok(())
}

async fn cmd_check_all(fix: bool, dry_run: bool, stats: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "phoneNumbers", "memberships", "metadata"]).await?;

    let mut results: Vec<(&str, usize)> = Vec::new();

    if stats {
        // Count-only mode: compute counts without printing individual items
        let count_names = |pred: &dyn Fn(&str) -> bool| -> usize {
            all_contacts.iter().filter(|p| {
                let name = person_name(p);
                !name.is_empty() && pred(name)
            }).count()
        };
        let count_phones = |pred: &dyn Fn(&str) -> bool| -> usize {
            let mut count = 0;
            for person in &all_contacts {
                if let Some(nums) = &person.phone_numbers {
                    count += nums.iter().filter(|pn| {
                        pn.value.as_deref().is_some_and(|v| pred(v))
                    }).count();
                }
            }
            count
        };

        results.push(("check-name-english", count_names(&|name| !is_english_name(name))));
        results.push(("check-name-caps", count_names(&|name| is_all_caps(name))));
        results.push(("check-phone-countrycode", count_phones(&|v| is_fixable_phone(v) && !has_country_code(v))));
        results.push(("check-phone-format", count_phones(&|v| is_fixable_phone(v) && !is_correct_phone_format(v))));
        results.push(("check-name-first-capital-letter", count_names(&|name| !starts_with_capital(name))));
        results.push(("check-name-order", all_contacts.iter().filter(|p| has_reversed_name(p)).count()));
        results.push(("check-contact-no-label", all_contacts.iter().filter(|p| !has_user_label(p)).count()));
        results.push(("check-phone-no-label", {
            let mut c = 0;
            for p in &all_contacts {
                if let Some(nums) = &p.phone_numbers {
                    c += nums.iter().filter(|pn| !phone_has_type(pn)).count();
                }
            }
            c
        }));
        results.push(("check-phone-label-english", {
            let mut c = 0;
            for p in &all_contacts {
                if let Some(nums) = &p.phone_numbers {
                    for pn in nums {
                        let label = pn.formatted_type.as_deref().or(pn.type_.as_deref()).unwrap_or("");
                        if !label.is_empty() && !label.chars().all(|ch| ch.is_ascii()) { c += 1; }
                    }
                }
            }
            c
        }));
        results.push(("check-email", {
            let mut c = 0;
            for p in &all_contacts {
                if let Some(emails) = &p.email_addresses {
                    c += emails.iter().filter(|e| e.value.as_deref().is_some_and(|v| !is_valid_email(v))).count();
                }
            }
            c
        }));
        results.push(("check-email-caps", {
            let mut c = 0;
            for p in &all_contacts {
                if let Some(emails) = &p.email_addresses {
                    c += emails.iter().filter(|e| e.value.as_deref().is_some_and(|v| v != v.to_lowercase().as_str())).count();
                }
            }
            c
        }));
        results.push(("check-duplicate-phones", {
            let mut c = 0;
            for p in &all_contacts {
                if let Some(nums) = &p.phone_numbers {
                    let values: Vec<&str> = nums.iter().filter_map(|pn| pn.value.as_deref()).collect();
                    let mut seen = std::collections::HashSet::new();
                    c += values.iter().filter(|v| !seen.insert(**v)).count();
                }
            }
            c
        }));
        results.push(("check-duplicate-emails", {
            let mut c = 0;
            for p in &all_contacts {
                if let Some(emails) = &p.email_addresses {
                    let values: Vec<&str> = emails.iter().filter_map(|e| e.value.as_deref()).collect();
                    let mut seen = std::collections::HashSet::new();
                    c += values.iter().filter(|v| !seen.insert(**v)).count();
                }
            }
            c
        }));
    } else {
        let non_english = check_name_issues(
            &hub, &all_contacts, |name| !is_english_name(name),
            fix, dry_run, "  ", Some("Non-English names (check-name-english)"),
        ).await?;
        results.push(("check-name-english", non_english));

        let all_caps = check_name_issues(
            &hub, &all_contacts, |name| is_all_caps(name),
            fix, dry_run, "  ", Some("All-caps names (check-name-caps)"),
        ).await?;
        results.push(("check-name-caps", all_caps));

        let country_owned = country.to_string();
        let no_country = check_phone_issues(
            &hub, &all_contacts,
            |v| is_fixable_phone(v) && !has_country_code(v),
            move |v| add_country_code(v, &country_owned),
            fix, dry_run, "  ", Some("Phones missing country code (check-phone-countrycode)"),
        ).await?;
        results.push(("check-phone-countrycode", no_country));

        let country_owned2 = country.to_string();
        let bad_format = check_phone_issues(
            &hub, &all_contacts,
            |v| is_fixable_phone(v) && !is_correct_phone_format(v),
            move |v| fix_phone_format(v, &country_owned2),
            fix, dry_run, "  ", Some("Phones not in +CC-NUMBER format (check-phone-format)"),
        ).await?;
        results.push(("check-phone-format", bad_format));

        let first_cap = check_name_issues(
            &hub, &all_contacts, |name| !starts_with_capital(name),
            fix, dry_run, "  ", Some("Names not starting with capital letter (check-name-first-capital-letter)"),
        ).await?;
        results.push(("check-name-first-capital-letter", first_cap));

        results.push(("check-name-order", check_name_order(&all_contacts, "  ", Some("Reversed name order (check-name-order)"))));
        results.push(("check-contact-no-label", check_no_label(&all_contacts, "  ", Some("Contacts without label (check-contact-no-label)"))));
        results.push(("check-phone-no-label", check_phone_no_label(&all_contacts, "  ", Some("Phones without label (check-phone-no-label)"))));
        results.push(("check-phone-label-english", check_phone_label_english(&all_contacts, "  ", Some("Non-English phone labels (check-phone-label-english)"))));
        results.push(("check-email", check_invalid_emails(&all_contacts, "  ", Some("Invalid emails (check-email)"))));
        results.push(("check-email-caps", check_email_caps(&all_contacts, "  ", Some("Emails with uppercase (check-email-caps)"))));
        results.push(("check-duplicate-phones", check_duplicate_phones(&all_contacts, "  ", Some("Duplicate phone numbers (check-duplicate-phones)"))));
        results.push(("check-duplicate-emails", check_duplicate_emails(&all_contacts, "  ", Some("Duplicate email addresses (check-duplicate-emails)"))));
    }

    // Check for empty labels (contact groups) — separate API call
    let mut all_groups: Vec<google_people1::api::ContactGroup> = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut request = hub.contact_groups().list();
        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }
        let (_response, result) = request.doit().await?;
        if let Some(groups) = result.contact_groups {
            all_groups.extend(groups);
        }
        page_token = result.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    let empty: Vec<_> = all_groups.iter().filter(|g| {
        g.member_count.unwrap_or(0) == 0
            && g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
    }).collect();
    if !stats && !empty.is_empty() {
        println!("=== Empty labels (check-labels-nophone) ({}) ===", empty.len());
        for group in &empty {
            let name = group.name.as_deref().unwrap_or("<unnamed>");
            println!("  {}", name);
        }
        println!();
    }
    results.push(("check-labels-nophone", empty.len()));

    let with_space: Vec<_> = all_groups.iter().filter(|g| {
        g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
            && g.name.as_deref().unwrap_or("").contains(' ')
    }).collect();
    if !stats && !with_space.is_empty() {
        println!("=== Labels with spaces (check-contact-label-space) ({}) ===", with_space.len());
        for group in &with_space {
            let name = group.name.as_deref().unwrap_or("<unnamed>");
            println!("  {}", name);
        }
        println!();
    }
    results.push(("check-contact-label-space", with_space.len()));

    if stats {
        let total: usize = results.iter().map(|(_, c)| c).sum();
        for (name, count) in &results {
            if *count > 0 {
                println!("{}: {}", name, count);
            }
        }
        if total == 0 {
            println!("All checks passed!");
        } else {
            println!("---");
            println!("Total: {}", total);
        }
    } else {
        let found_any = results.iter().any(|(_, c)| *c > 0);
        if !found_any {
            println!("All checks passed!");
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
        Commands::Auth { no_browser, force } => cmd_auth(no_browser, force).await?,
        Commands::List { emails, labels } => cmd_list(emails, labels).await?,
        Commands::CheckNameEnglish { fix, dry_run } => cmd_check_english(fix, dry_run).await?,
        Commands::CheckNameCaps { fix, dry_run } => cmd_check_caps(fix, dry_run).await?,
        Commands::CheckNameFirstCapitalLetter { fix, dry_run } => cmd_check_first_capital_letter(fix, dry_run).await?,
        Commands::CheckNameOrder { fix, dry_run } => cmd_check_name_order(fix, dry_run).await?,
        Commands::CheckPhoneCountrycode { fix, dry_run, ref country } => cmd_check_phone_countrycode(fix, dry_run, country).await?,
        Commands::CheckPhoneFormat { fix, dry_run, ref country } => cmd_check_phone_format(fix, dry_run, country).await?,
        Commands::CheckContactNoLabel => cmd_check_contact_no_label().await?,
        Commands::CheckPhoneNoLabel => cmd_check_phone_no_label().await?,
        Commands::CheckPhoneLabelEnglish { fix, dry_run } => cmd_check_phone_label_english(fix, dry_run).await?,
        Commands::CheckEmail => cmd_check_email().await?,
        Commands::CheckEmailCaps { fix, dry_run } => cmd_check_email_caps(fix, dry_run).await?,
        Commands::CheckDuplicateEmails { fix, dry_run } => cmd_check_duplicate_emails(fix, dry_run).await?,
        Commands::CheckDuplicatePhones { fix, dry_run } => cmd_check_duplicate_phones(fix, dry_run).await?,
        Commands::CheckLabelsNophone { fix, dry_run } => cmd_check_labels_nophone(fix, dry_run).await?,
        Commands::CheckContactLabelSpace { fix, dry_run } => cmd_check_contact_label_space(fix, dry_run).await?,
        Commands::ShowContact { ref name } => cmd_show_contact(name).await?,
        Commands::ShowPhoneLabels => cmd_show_phone_labels().await?,
        Commands::ShowContactLabels => cmd_show_contact_labels().await?,
        Commands::CheckAll { fix, dry_run, stats, ref country } => cmd_check_all(fix, dry_run, stats, country).await?,
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
        assert!(matches!(cli.command, Commands::Auth { no_browser: false, force: false }));
    }

    #[test]
    fn test_cli_auth_no_browser() {
        let cli = Cli::parse_from(["rscontacts", "auth", "--no-browser"]);
        assert!(matches!(cli.command, Commands::Auth { no_browser: true, force: false }));
    }

    #[test]
    fn test_cli_auth_force() {
        let cli = Cli::parse_from(["rscontacts", "auth", "--force"]);
        assert!(matches!(cli.command, Commands::Auth { no_browser: false, force: true }));
    }

    #[test]
    fn test_cli_list_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "list"]);
        assert!(matches!(cli.command, Commands::List { .. }));
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
    fn test_cli_check_name_english_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-name-english"]);
        assert!(matches!(cli.command, Commands::CheckNameEnglish { fix: false, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_english_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-name-english", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckNameEnglish { fix: true, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_english_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "check-name-english", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::CheckNameEnglish { fix: true, dry_run: true }));
    }

    #[test]
    fn test_cli_check_name_caps_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-name-caps"]);
        assert!(matches!(cli.command, Commands::CheckNameCaps { fix: false, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_caps_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-name-caps", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckNameCaps { fix: true, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_caps_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "check-name-caps", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::CheckNameCaps { fix: true, dry_run: true }));
    }

    #[test]
    fn test_cli_check_all_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-all"]);
        assert!(matches!(cli.command, Commands::CheckAll { fix: false, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_all_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-all", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckAll { fix: true, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_all_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "check-all", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::CheckAll { fix: true, dry_run: true, .. }));
    }

    #[test]
    fn test_cli_check_all_custom_country() {
        let cli = Cli::parse_from(["rscontacts", "check-all", "--country", "1"]);
        if let Commands::CheckAll { country, .. } = cli.command {
            assert_eq!(country, "1");
        } else {
            panic!("wrong command");
        }
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
        let cli = Cli::parse_from(["rscontacts", "check-phone-countrycode"]);
        assert!(matches!(cli.command, Commands::CheckPhoneCountrycode { fix: false, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_phone_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-phone-countrycode", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckPhoneCountrycode { fix: true, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_phone_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "check-phone-countrycode", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::CheckPhoneCountrycode { fix: true, dry_run: true, .. }));
    }

    #[test]
    fn test_cli_check_phone_custom_country() {
        let cli = Cli::parse_from(["rscontacts", "check-phone-countrycode", "--country", "1"]);
        if let Commands::CheckPhoneCountrycode { country, .. } = cli.command {
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
    fn test_is_correct_phone_format() {
        assert!(is_correct_phone_format("+972-505665636"));
        assert!(is_correct_phone_format("+1-5551234567"));
        assert!(is_correct_phone_format("+44-2079460958"));
    }

    #[test]
    fn test_is_correct_phone_format_invalid() {
        assert!(!is_correct_phone_format("+972505665636")); // missing dash
        assert!(!is_correct_phone_format("0505665636")); // no country code
        assert!(!is_correct_phone_format("+972-505-665636")); // extra dash
        assert!(!is_correct_phone_format("+972 505665636")); // space instead of dash
        assert!(!is_correct_phone_format("00972505665636")); // 00 prefix
    }

    #[test]
    fn test_fix_phone_format() {
        assert_eq!(fix_phone_format("+972505665636", "972"), "+972-505665636");
        assert_eq!(fix_phone_format("+972-50-5665636", "972"), "+972-505665636");
        assert_eq!(fix_phone_format("0505665636", "972"), "+972-505665636");
        assert_eq!(fix_phone_format("00972505665636", "972"), "+972-505665636");
        assert_eq!(fix_phone_format("+972 50 566 5636", "972"), "+972-505665636");
        // Single-digit country code (Russia)
        assert_eq!(fix_phone_format("+79268335991", "972"), "+7-9268335991");
        // Two-digit country code (UK)
        assert_eq!(fix_phone_format("+442079460958", "972"), "+44-2079460958");
        // US/Canada
        assert_eq!(fix_phone_format("+15551234567", "972"), "+1-5551234567");
    }

    #[test]
    fn test_detect_country_code() {
        assert_eq!(detect_country_code("79268335991"), Some(1));  // Russia +7
        assert_eq!(detect_country_code("442079460958"), Some(2)); // UK +44
        assert_eq!(detect_country_code("972505665636"), Some(3)); // Israel +972
        assert_eq!(detect_country_code("15551234567"), Some(1));  // US +1
        assert_eq!(detect_country_code(""), None);
    }

    #[test]
    fn test_config_dir_ends_with_rscontacts() {
        let dir = config_dir();
        assert!(dir.ends_with("rscontacts"));
        assert!(dir.to_str().unwrap().contains(".config"));
    }

    #[test]
    fn test_is_valid_email_valid() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("first.last@domain.co.uk"));
        assert!(is_valid_email("test+tag@gmail.com"));
    }

    #[test]
    fn test_is_valid_email_invalid() {
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("noatsign"));
        assert!(!is_valid_email("@nodomain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@nodot"));
        assert!(!is_valid_email("user@domain.x"));
    }

    #[test]
    fn test_token_cache_path_is_in_config_dir() {
        let path = token_cache_path();
        assert!(path.ends_with("token_cache.json"));
        assert!(path.to_str().unwrap().contains(".config/rscontacts"));
    }
}
