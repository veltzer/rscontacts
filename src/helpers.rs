use google_people1::api::ListConnectionsResponse;
use google_people1::{FieldMask, PeopleService};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::LazyLock;
use std::time::Duration;

pub const MUTATE_DELAY: Duration = Duration::from_millis(500);

#[derive(serde::Deserialize, Default, Debug)]
pub struct Config {
    #[serde(default, rename = "check-all")]
    pub check_all: CheckAllConfig,
    #[serde(default, rename = "check-contact-firstname-regexp")]
    pub check_contact_firstname_regexp: NameRegexpConfig,
    #[serde(default, rename = "check-contact-lastname-regexp")]
    pub check_contact_lastname_regexp: NameRegexpConfig,
    #[serde(default, rename = "check-contact-suffix-regexp")]
    pub check_contact_suffix_regexp: NameRegexpConfig,
}

#[derive(serde::Deserialize, Default, Debug)]
pub struct CheckAllConfig {
    #[serde(default)]
    pub skip: Vec<String>,
}

#[derive(serde::Deserialize, Default, Debug)]
pub struct NameRegexpConfig {
    #[serde(default)]
    pub allow: Option<String>,
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load_config() -> Config {
    let path = config_path();
    if !path.exists() {
        return Config::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {}", path.display(), e);
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("Warning: failed to read {}: {}", path.display(), e);
            Config::default()
        }
    }
}

pub fn config_dir() -> PathBuf {
    let mut dir = dirs::home_dir().expect("Could not determine home directory");
    dir.push(".config");
    dir.push("rscontacts");
    std::fs::create_dir_all(&dir).expect("Could not create config directory");
    dir
}

pub fn credentials_path() -> PathBuf {
    let path = config_dir().join("credentials.json");
    if !path.exists() {
        eprintln!("Error: credentials.json not found at {}", path.display());
        eprintln!("Download OAuth2 credentials from Google Cloud Console and place them there.");
        std::process::exit(1);
    }
    path
}

pub fn token_cache_path() -> PathBuf {
    config_dir().join("token_cache.json")
}

pub fn is_valid_email(email: &str) -> bool {
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

pub fn get_phone_label(pn: &google_people1::api::PhoneNumber) -> &str {
    pn.formatted_type.as_deref()
        .or(pn.type_.as_deref())
        .unwrap_or("")
}

pub fn find_duplicates<'a>(values: &[&'a str]) -> Vec<&'a str> {
    let mut seen = std::collections::HashSet::new();
    values.iter().filter(|v| !seen.insert(**v)).copied().collect()
}

pub fn phone_has_type(pn: &google_people1::api::PhoneNumber) -> bool {
    !pn.type_.as_deref().unwrap_or("").is_empty()
        || !pn.formatted_type.as_deref().unwrap_or("").is_empty()
}

pub fn starts_with_capital(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

pub fn is_all_caps(name: &str) -> bool {
    let alpha_chars: String = name.chars().filter(|c| c.is_alphabetic()).collect();
    alpha_chars.len() >= 2 && alpha_chars == alpha_chars.to_uppercase()
}

pub fn is_english_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii() || c == '\u{200f}' || c == '\u{200e}')
}

pub fn is_fixable_phone(phone: &str) -> bool {
    let trimmed = phone.trim();
    let digits: Vec<char> = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    !trimmed.is_empty()
        && !trimmed.contains(|c: char| c.is_alphabetic())
        && !trimmed.starts_with('*')
        && digits.len() > 3
}

pub fn has_country_code(phone: &str) -> bool {
    let trimmed = phone.trim();
    trimmed.starts_with('+') || trimmed.starts_with("00")
}

pub fn add_country_code(phone: &str, country: &str) -> String {
    let trimmed = phone.trim();
    let without_leading_zero = trimmed.strip_prefix('0').unwrap_or(trimmed);
    format!("+{}{}", country, without_leading_zero)
}

pub fn is_correct_phone_format(phone: &str) -> bool {
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
        && COUNTRY_CODE_SET.contains(cc)
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
        && !rest[dash_pos + 1..].contains('-')
}

static COUNTRY_CODE_SET: LazyLock<std::collections::HashSet<&'static str>> = LazyLock::new(|| {
    COUNTRY_CODES.iter().copied().collect()
});

pub const COUNTRY_CODES: &[&str] = &[
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

pub fn detect_country_code(digits: &str) -> Option<usize> {
    // Try longest match first (3 digits, then 2, then 1)
    for len in (1..=3).rev() {
        if digits.len() > len {
            let prefix = &digits[..len];
            if COUNTRY_CODE_SET.contains(prefix) {
                return Some(len);
            }
        }
    }
    None
}

pub fn fix_phone_format(phone: &str, country: &str) -> String {
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

// --- API helper functions ---

pub type HubType = PeopleService<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

pub async fn fetch_all_contacts(hub: &HubType, fields: &[&str]) -> Result<Vec<google_people1::api::Person>, Box<dyn std::error::Error>> {
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

pub fn person_name(person: &google_people1::api::Person) -> &str {
    person
        .names
        .as_ref()
        .and_then(|names| names.first())
        .and_then(|n| n.display_name.as_deref())
        .unwrap_or("")
}

pub fn person_display_name(person: &google_people1::api::Person) -> &str {
    let name = person_name(person);
    if name.is_empty() { "<no name>" } else { name }
}

pub fn person_email(person: &google_people1::api::Person) -> &str {
    person
        .email_addresses
        .as_ref()
        .and_then(|emails| emails.first())
        .and_then(|e| e.value.as_deref())
        .unwrap_or("")
}

pub async fn interactive_name_fix(hub: &HubType, person: &google_people1::api::Person, name: &str) -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn update_phone_numbers<F>(hub: &HubType, person: &google_people1::api::Person, transform: F) -> Result<(), Box<dyn std::error::Error>>
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

pub async fn fetch_all_contact_groups(hub: &HubType) -> Result<Vec<google_people1::api::ContactGroup>, Box<dyn std::error::Error>> {
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

    Ok(all_groups)
}

// --- Auth delegates ---

pub struct NoInteractionDelegate;

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

pub struct BrowserFlowDelegate;

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

pub fn build_connector() -> Result<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, Box<dyn std::error::Error>> {
    Ok(hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http2()
        .build())
}

pub async fn build_hub() -> Result<HubType, Box<dyn std::error::Error>> {
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

// --- Prompt helpers ---

pub fn prompt_firstname_fix_action(given: &str, family: &str) -> Result<char, Box<dyn std::error::Error>> {
    use std::io::Write;
    let has_split = split_alpha_numeric(given).is_some();
    loop {
        if has_split {
            let (alpha, numeric) = split_alpha_numeric(given).unwrap();
            eprint!("  s[w]ap lastname \"{}\" to firstname / s[p]lit \"{}\" -> firstname \"{}\", suffix \"{}\" / [c]ompany / [r]ename / [d]elete / [s]kip? ", family, given, alpha, numeric);
        } else {
            eprint!("  s[w]ap lastname \"{}\" to firstname / [c]ompany / [r]ename / [d]elete / [s]kip? ", family);
        }
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim().chars().next() {
            Some(c @ ('w' | 'c' | 'r' | 'd' | 's')) => return Ok(c),
            Some('p') if has_split => return Ok('p'),
            _ => {
                if has_split {
                    eprintln!("  Invalid choice. Enter w, p, c, r, d, or s.");
                } else {
                    eprintln!("  Invalid choice. Enter w, c, r, d, or s.");
                }
            }
        }
    }
}

pub fn prompt_fix_action(_name: &str) -> Result<char, Box<dyn std::error::Error>> {
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

pub fn prompt_new_name(old_name: &str) -> Result<String, Box<dyn std::error::Error>> {
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

pub fn prompt_new_name_with_default(old_name: &str, suggested: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Write;
    eprint!("  New name for \"{}\" [Enter for \"{}\"]: ", old_name, suggested);
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(suggested.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn prompt_yes_no(message: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use std::io::Write;
    loop {
        eprint!("  {} [y/n] ", message);
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

pub fn prompt_phone_label_fix(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
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

pub fn prompt_rename_label(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
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

// --- Display helpers ---

pub fn print_name_with_email(name: &str, email: &str, prefix: &str) {
    if !email.is_empty() {
        println!("{}{} | {}", prefix, name, email);
    } else {
        println!("{}{}", prefix, name);
    }
}

pub fn print_phone_fix(name: &str, phone: &str, fixed: &str, fix: bool, dry_run: bool, prefix: &str) {
    if fix || dry_run {
        println!("{}{} | {} -> {}", prefix, name, phone, fixed);
    } else {
        println!("{}{} | {}", prefix, name, phone);
    }
}

pub fn is_numeric_string(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Extract the "base name" from a display name by stripping a trailing numeric suffix.
/// E.g., "Mike 1" -> "Mike", "Mike" -> "Mike", "Mike 2" -> "Mike".
/// Returns (base_name, optional_suffix_number).
pub fn split_name_suffix(display_name: &str) -> (&str, Option<u32>) {
    if let Some(pos) = display_name.rfind(' ') {
        let after = &display_name[pos + 1..];
        if is_numeric_string(after) {
            if let Ok(n) = after.parse::<u32>() {
                return (display_name[..pos].trim_end(), Some(n));
            }
        }
    }
    (display_name, None)
}

/// Split a string into its alpha prefix and numeric suffix.
/// E.g. "Mike2" -> Some(("Mike", "2")), "Mike" -> None, "123" -> None
pub fn split_alpha_numeric(s: &str) -> Option<(&str, &str)> {
    let num_start = s.find(|c: char| c.is_ascii_digit())?;
    if num_start == 0 {
        return None;
    }
    let alpha = &s[..num_start];
    let numeric = &s[num_start..];
    if numeric.chars().all(|c| c.is_ascii_digit()) {
        Some((alpha, numeric))
    } else {
        None
    }
}

pub fn has_reversed_name(person: &google_people1::api::Person) -> bool {
    let name = person_name(person);
    if let Some((_, after)) = name.split_once(',') {
        let after = after.trim();
        // Don't flag "Name, 2" — that's a numeric suffix, not a reversed name
        !is_numeric_string(after)
    } else {
        false
    }
}

pub fn compute_fixed_name(display_name: &str) -> String {
    if let Some((family, given)) = display_name.split_once(',') {
        let given = given.trim();
        let family = family.trim();
        if !given.is_empty() && !family.is_empty() {
            return format!("{} {}", given, family);
        }
    }
    display_name.to_string()
}

pub fn is_starred(person: &google_people1::api::Person) -> bool {
    person.memberships.as_ref().is_some_and(|memberships| {
        memberships.iter().any(|m| {
            m.contact_group_membership.as_ref().is_some_and(|cgm| {
                cgm.contact_group_resource_name.as_deref() == Some("contactGroups/starred")
            })
        })
    })
}

/// System contact groups that should be ignored when checking if a contact has a label.
///
/// Google Contacts has two kinds of system contact groups:
/// - **Active:** `myContacts` (all contacts) and `starred` (starred contacts). These are
///   managed automatically and don't represent user-assigned labels.
/// - **Deprecated:** `family`, `friends`, `coworkers`, `chatBuddies`, `all`, `blocked`.
///   These were predefined by Google but are now deprecated in the People API. You can
///   remove contacts from them but **cannot add** new contacts to them (the API returns
///   400 Bad Request: "Cannot add contacts to deprecated system contact group"). They
///   still appear in the web UI but are not functional for labeling purposes. Users should
///   create their own user contact groups (e.g. "Family") instead.
///
/// None of these system groups count as real user-assigned labels.
pub const IGNORED_SYSTEM_GROUPS: &[&str] = &[
    "contactGroups/myContacts",
    "contactGroups/starred",
    "contactGroups/family",
    "contactGroups/friends",
    "contactGroups/coworkers",
    "contactGroups/chatBuddies",
    "contactGroups/all",
    "contactGroups/blocked",
];

pub fn has_user_label(person: &google_people1::api::Person) -> bool {
    person.memberships.as_ref().is_some_and(|memberships| {
        memberships.iter().any(|m| {
            m.contact_group_membership.as_ref().is_some_and(|cgm| {
                let rn = cgm.contact_group_resource_name.as_deref().unwrap_or("");
                !rn.is_empty() && !IGNORED_SYSTEM_GROUPS.contains(&rn)
            })
        })
    })
}
