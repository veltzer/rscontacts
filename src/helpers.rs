use google_people1::api::ListConnectionsResponse;
use google_people1::{FieldMask, PeopleService};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const MUTATE_DELAY: Duration = Duration::from_millis(500);

static TRANSPORT_ERRORS: AtomicBool = AtomicBool::new(false);
const MAX_RETRIES: u32 = 3;
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn set_transport_errors(enabled: bool) {
    TRANSPORT_ERRORS.store(enabled, Ordering::Relaxed);
}

fn is_transient_status(status: u16) -> bool {
    matches!(status, 429 | 502 | 503 | 504)
}

/// Retry an API call on transient HTTP errors (429, 502, 503, 504).
/// The closure must rebuild the request each time since `.doit()` consumes the builder.
pub async fn retry_api<F, Fut, T>(mut make_request: F) -> Result<T, google_people1::Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, google_people1::Error>>,
{
    let verbose = TRANSPORT_ERRORS.load(Ordering::Relaxed);
    for attempt in 0..=MAX_RETRIES {
        let result = tokio::time::timeout(REQUEST_TIMEOUT, make_request()).await;
        match result {
            Err(_elapsed) => {
                if attempt < MAX_RETRIES {
                    let delay = RETRY_DELAYS[attempt as usize];
                    if verbose {
                        eprintln!("  [transport] request timed out after {}s - retrying in {}s (attempt {}/{})", REQUEST_TIMEOUT.as_secs(), delay.as_secs(), attempt + 1, MAX_RETRIES);
                    }
                    tokio::time::sleep(delay).await;
                } else {
                    return Err(google_people1::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("request timed out after {}s (exhausted all retries)", REQUEST_TIMEOUT.as_secs()),
                    )));
                }
            }
            Ok(Ok(val)) => return Ok(val),
            Ok(Err(google_people1::Error::Failure(ref resp))) if attempt < MAX_RETRIES && is_transient_status(resp.status().as_u16()) => {
                let status = resp.status();
                let delay = RETRY_DELAYS[attempt as usize];
                if verbose {
                    eprintln!("  [transport] HTTP {} - retrying in {}s (attempt {}/{})", status, delay.as_secs(), attempt + 1, MAX_RETRIES);
                }
                tokio::time::sleep(delay).await;
            }
            Ok(Err(e)) => return Err(e),
        }
    }
    unreachable!()
}

/// Convenience macro to wrap a `.doit().await` call with retry logic.
/// The expression must be a builder chain ending before `.doit()`.
/// It is re-evaluated on each retry, so it must not consume owned values
/// that aren't recreated (use `.clone()` for owned parameters).
///
/// Usage: `doit_retry!(hub.people().connections_list("people/me").person_fields(mask))`
#[macro_export]
macro_rules! doit_retry {
    ($expr:expr) => {
        $crate::helpers::retry_api(|| async { $expr.doit().await }).await
    };
}
pub const PHONE_LABEL_OPTIONS: &[&str] = &["mobile", "home", "work", "main", "other"];

#[derive(serde::Deserialize, Default, Debug)]
pub struct Config {
    #[serde(default, rename = "check-all")]
    pub check_all: CheckAllConfig,
    #[serde(default, rename = "check-contact-given-name-regexp")]
    pub check_contact_given_name_regexp: NameRegexpConfig,
    #[serde(default, rename = "check-contact-family-name-regexp")]
    pub check_contact_family_name_regexp: NameRegexpConfig,
    #[serde(default, rename = "check-contact-suffix-regexp")]
    pub check_contact_suffix_regexp: NameRegexpConfig,
    #[serde(default, rename = "check-contact-label-regexp")]
    pub check_contact_label_regexp: NameRegexpConfig,
    #[serde(default, rename = "check-contact-name-is-company")]
    pub check_contact_name_is_company: CompanyNamesConfig,
    #[serde(default, rename = "check-contact-given-name-known")]
    pub check_contact_given_name_known: GivenNamesConfig,
}

#[derive(serde::Deserialize, Default, Debug)]
pub struct GivenNamesConfig {
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(serde::Deserialize, Default, Debug)]
pub struct CompanyNamesConfig {
    #[serde(default)]
    pub companies: Vec<String>,
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

pub fn get_email_label(e: &google_people1::api::EmailAddress) -> &str {
    e.formatted_type.as_deref()
        .or(e.type_.as_deref())
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

/// Map from phone country code (digits only, no +) to country name.
/// Used by `check-phone-country-label` to determine the expected `country:<Name>` label.
pub static COUNTRY_CODE_TO_NAME: LazyLock<std::collections::HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        ("1", "USA"),
        ("7", "Russia"),
        ("20", "Egypt"),
        ("27", "South Africa"),
        ("30", "Greece"),
        ("31", "Netherlands"),
        ("32", "Belgium"),
        ("33", "France"),
        ("34", "Spain"),
        ("36", "Hungary"),
        ("39", "Italy"),
        ("40", "Romania"),
        ("41", "Switzerland"),
        ("43", "Austria"),
        ("44", "UK"),
        ("45", "Denmark"),
        ("46", "Sweden"),
        ("47", "Norway"),
        ("48", "Poland"),
        ("49", "Germany"),
        ("51", "Peru"),
        ("52", "Mexico"),
        ("53", "Cuba"),
        ("54", "Argentina"),
        ("55", "Brazil"),
        ("56", "Chile"),
        ("57", "Colombia"),
        ("58", "Venezuela"),
        ("60", "Malaysia"),
        ("61", "Australia"),
        ("62", "Indonesia"),
        ("63", "Philippines"),
        ("64", "New Zealand"),
        ("65", "Singapore"),
        ("66", "Thailand"),
        ("81", "Japan"),
        ("82", "South Korea"),
        ("84", "Vietnam"),
        ("86", "China"),
        ("90", "Turkey"),
        ("91", "India"),
        ("92", "Pakistan"),
        ("93", "Afghanistan"),
        ("94", "Sri Lanka"),
        ("95", "Myanmar"),
        ("98", "Iran"),
        ("212", "Morocco"),
        ("213", "Algeria"),
        ("216", "Tunisia"),
        ("218", "Libya"),
        ("220", "Gambia"),
        ("221", "Senegal"),
        ("222", "Mauritania"),
        ("223", "Mali"),
        ("224", "Guinea"),
        ("225", "Ivory Coast"),
        ("226", "Burkina Faso"),
        ("229", "Benin"),
        ("230", "Mauritius"),
        ("231", "Liberia"),
        ("232", "Sierra Leone"),
        ("233", "Ghana"),
        ("234", "Nigeria"),
        ("235", "Chad"),
        ("236", "Central African Republic"),
        ("238", "Cape Verde"),
        ("239", "Sao Tome"),
        ("240", "Equatorial Guinea"),
        ("241", "Gabon"),
        ("242", "Congo"),
        ("243", "DR Congo"),
        ("244", "Angola"),
        ("245", "Guinea-Bissau"),
        ("246", "Diego Garcia"),
        ("247", "Ascension Island"),
        ("248", "Seychelles"),
        ("249", "Sudan"),
        ("250", "Rwanda"),
        ("251", "Ethiopia"),
        ("252", "Somalia"),
        ("253", "Djibouti"),
        ("254", "Kenya"),
        ("255", "Tanzania"),
        ("256", "Uganda"),
        ("257", "Burundi"),
        ("258", "Mozambique"),
        ("260", "Zambia"),
        ("262", "Reunion"),
        ("263", "Zimbabwe"),
        ("264", "Namibia"),
        ("265", "Malawi"),
        ("266", "Lesotho"),
        ("267", "Botswana"),
        ("268", "Eswatini"),
        ("269", "Comoros"),
        ("290", "Saint Helena"),
        ("297", "Aruba"),
        ("298", "Faroe Islands"),
        ("299", "Greenland"),
        ("350", "Gibraltar"),
        ("351", "Portugal"),
        ("352", "Luxembourg"),
        ("353", "Ireland"),
        ("354", "Iceland"),
        ("355", "Albania"),
        ("356", "Malta"),
        ("357", "Cyprus"),
        ("358", "Finland"),
        ("359", "Bulgaria"),
        ("370", "Lithuania"),
        ("371", "Latvia"),
        ("372", "Estonia"),
        ("373", "Moldova"),
        ("374", "Armenia"),
        ("375", "Belarus"),
        ("376", "Andorra"),
        ("377", "Monaco"),
        ("378", "San Marino"),
        ("379", "Vatican"),
        ("380", "Ukraine"),
        ("381", "Serbia"),
        ("382", "Montenegro"),
        ("385", "Croatia"),
        ("386", "Slovenia"),
        ("387", "Bosnia"),
        ("388", "EU"),
        ("389", "North Macedonia"),
        ("420", "Czech Republic"),
        ("423", "Liechtenstein"),
        ("500", "Falkland Islands"),
        ("501", "Belize"),
        ("502", "Guatemala"),
        ("503", "El Salvador"),
        ("504", "Honduras"),
        ("505", "Nicaragua"),
        ("506", "Costa Rica"),
        ("507", "Panama"),
        ("508", "Saint Pierre"),
        ("509", "Haiti"),
        ("590", "Guadeloupe"),
        ("591", "Bolivia"),
        ("592", "Guyana"),
        ("593", "Ecuador"),
        ("594", "French Guiana"),
        ("595", "Paraguay"),
        ("596", "Martinique"),
        ("597", "Suriname"),
        ("598", "Uruguay"),
        ("599", "Curacao"),
        ("672", "Norfolk Island"),
        ("673", "Brunei"),
        ("674", "Nauru"),
        ("675", "Papua New Guinea"),
        ("676", "Tonga"),
        ("677", "Solomon Islands"),
        ("678", "Vanuatu"),
        ("679", "Fiji"),
        ("680", "Palau"),
        ("681", "Wallis and Futuna"),
        ("682", "Cook Islands"),
        ("683", "Niue"),
        ("684", "American Samoa"),
        ("685", "Samoa"),
        ("686", "Kiribati"),
        ("687", "New Caledonia"),
        ("688", "Tuvalu"),
        ("689", "French Polynesia"),
        ("690", "Tokelau"),
        ("691", "Micronesia"),
        ("692", "Marshall Islands"),
        ("800", "International Freephone"),
        ("808", "Shared Cost"),
        ("850", "North Korea"),
        ("852", "Hong Kong"),
        ("853", "Macau"),
        ("855", "Cambodia"),
        ("856", "Laos"),
        ("870", "Inmarsat"),
        ("878", "Universal Personal"),
        ("880", "Bangladesh"),
        ("881", "Global Mobile Satellite"),
        ("882", "International Networks"),
        ("883", "International Networks"),
        ("886", "Taiwan"),
        ("888", "Telecommunications for Disaster Relief"),
        ("891", "International"),
        ("900", "International Premium Rate"),
        ("960", "Maldives"),
        ("961", "Lebanon"),
        ("962", "Jordan"),
        ("963", "Syria"),
        ("964", "Iraq"),
        ("965", "Kuwait"),
        ("966", "Saudi Arabia"),
        ("967", "Yemen"),
        ("968", "Oman"),
        ("970", "Palestine"),
        ("971", "UAE"),
        ("972", "Israel"),
        ("973", "Bahrain"),
        ("974", "Qatar"),
        ("975", "Bhutan"),
        ("976", "Mongolia"),
        ("977", "Nepal"),
        ("979", "International Premium Rate"),
        ("992", "Tajikistan"),
        ("993", "Turkmenistan"),
        ("994", "Azerbaijan"),
        ("995", "Georgia"),
        ("996", "Kyrgyzstan"),
        ("998", "Uzbekistan"),
    ].into_iter().collect()
});

/// Extract the country code digits from a phone number in +CC-NUMBER or +CCNUMBER format.
/// Returns None if the phone doesn't start with '+' or has no recognizable country code.
pub fn extract_country_code(phone: &str) -> Option<&'static str> {
    let trimmed = phone.trim();
    if !trimmed.starts_with('+') && !trimmed.starts_with("00") {
        return None;
    }
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    let pure_digits = if trimmed.starts_with("00") {
        &digits[2..]
    } else {
        &digits[..]
    };
    let cc_len = detect_country_code(pure_digits)?;
    let cc = &pure_digits[..cc_len];
    // Return the static str from COUNTRY_CODE_TO_NAME keys
    COUNTRY_CODE_TO_NAME.get_key_value(cc).map(|(k, _)| *k)
}

/// Get the country name for a phone number, if it has a recognized country code.
pub fn phone_country_name(phone: &str) -> Option<&'static str> {
    let cc = extract_country_code(phone)?;
    COUNTRY_CODE_TO_NAME.get(cc).copied()
}

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
        let (_response, result): (_, ListConnectionsResponse) = retry_api(|| {
            let mut req = hub
                .people()
                .connections_list("people/me")
                .person_fields(FieldMask::new::<&str>(fields));
            if let Some(ref token) = page_token {
                req = req.page_token(token);
            }
            async { req.doit().await }
        }).await?;

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

pub fn person_name(person: &google_people1::api::Person) -> String {
    let names = person.names.as_ref().and_then(|n| n.first());
    let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
    let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");
    let suffix = names.and_then(|n| n.honorific_suffix.as_deref()).unwrap_or("");
    let company = person.organizations.as_ref()
        .and_then(|orgs| orgs.first())
        .and_then(|o| o.name.as_deref())
        .unwrap_or("");
    let mut name_parts = Vec::new();
    if !given.is_empty() { name_parts.push(given.to_string()); }
    if !family.is_empty() { name_parts.push(family.to_string()); }
    if !suffix.is_empty() { name_parts.push(suffix.to_string()); }
    if !company.is_empty() {
        if name_parts.is_empty() {
            name_parts.push(company.to_string());
        } else {
            name_parts.push(format!("({})", company));
        }
    }
    name_parts.join(" ")
}

/// Returns given + family name only (no suffix, no company).
pub fn person_base_name(person: &google_people1::api::Person) -> String {
    let names = person.names.as_ref().and_then(|n| n.first());
    let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
    let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");
    let mut parts = Vec::new();
    if !given.is_empty() { parts.push(given); }
    if !family.is_empty() { parts.push(family); }
    parts.join(" ")
}

/// Format a person as a pipe-delimited line showing all non-empty fields.
/// This is the canonical way to display a contact — used by `list` and all checks.
/// `group_names` is optional; when provided, labels are included.
pub fn format_person_line(person: &google_people1::api::Person, group_names: Option<&std::collections::HashMap<String, String>>) -> String {
    let names = person.names.as_ref().and_then(|n| n.first());
    let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
    let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");
    let middle = names.and_then(|n| n.middle_name.as_deref()).unwrap_or("");
    let suffix = names.and_then(|n| n.honorific_suffix.as_deref()).unwrap_or("");
    let prefix_name = names.and_then(|n| n.honorific_prefix.as_deref()).unwrap_or("");
    let company = person.organizations.as_ref()
        .and_then(|orgs| orgs.first())
        .and_then(|o| o.name.as_deref())
        .unwrap_or("");
    let title = person.organizations.as_ref()
        .and_then(|orgs| orgs.first())
        .and_then(|o| o.title.as_deref())
        .unwrap_or("");
    let department = person.organizations.as_ref()
        .and_then(|orgs| orgs.first())
        .and_then(|o| o.department.as_deref())
        .unwrap_or("");

    let mut parts = Vec::new();
    if !prefix_name.is_empty() { parts.push(format!("prefix: {}", prefix_name)); }
    if !given.is_empty() { parts.push(format!("given: {}", given)); }
    if !middle.is_empty() { parts.push(format!("middle: {}", middle)); }
    if !family.is_empty() { parts.push(format!("family: {}", family)); }
    if !suffix.is_empty() { parts.push(format!("suffix: {}", suffix)); }
    if !company.is_empty() { parts.push(format!("company: {}", company)); }
    if !title.is_empty() { parts.push(format!("title: {}", title)); }
    if !department.is_empty() { parts.push(format!("dept: {}", department)); }

    if let Some(nicknames) = &person.nicknames {
        for n in nicknames {
            if let Some(val) = &n.value {
                parts.push(format!("nickname: {}", val));
            }
        }
    }

    if let Some(email_addrs) = &person.email_addresses {
        for e in email_addrs {
            if let Some(val) = e.value.as_deref()
                && !val.is_empty() {
                    let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("");
                    if t.is_empty() {
                        parts.push(format!("email: {}", val));
                    } else {
                        parts.push(format!("email: {} [{}]", val, t));
                    }
                }
        }
    }

    if let Some(phones) = &person.phone_numbers {
        for p in phones {
            if let Some(val) = p.value.as_deref()
                && !val.is_empty() {
                    let t = p.formatted_type.as_deref().or(p.type_.as_deref()).unwrap_or("");
                    if t.is_empty() {
                        parts.push(format!("phone: {}", val));
                    } else {
                        parts.push(format!("phone: {} [{}]", val, t));
                    }
                }
        }
    }

    if let Some(group_names) = group_names {
        let contact_labels = person_labels(person, group_names);
        if !contact_labels.is_empty() {
            parts.push(format!("labels: [{}]", contact_labels.join(", ")));
        }
    }

    if parts.is_empty() {
        "<no fields>".to_string()
    } else {
        parts.join(" | ")
    }
}


pub fn person_display_name(person: &google_people1::api::Person) -> String {
    let name = person_name(person);
    if name.is_empty() { "<no name>".to_string() } else { name }
}

pub fn get_resource_name(person: &google_people1::api::Person) -> Result<&str, String> {
    person.resource_name.as_deref()
        .ok_or_else(|| format!("Contact missing resource name for \"{}\"", person_display_name(person)))
}

pub fn build_group_name_map(groups: &[google_people1::api::ContactGroup]) -> std::collections::HashMap<String, String> {
    groups.iter()
        .filter_map(|g| {
            let rn = g.resource_name.as_deref()?;
            let name = g.name.as_deref()?;
            Some((rn.to_string(), name.to_string()))
        })
        .collect()
}

pub fn person_labels(person: &google_people1::api::Person, group_names: &std::collections::HashMap<String, String>) -> Vec<String> {
    person.memberships.as_ref()
        .map(|memberships| {
            memberships.iter().filter_map(|m| {
                let rn = m.contact_group_membership.as_ref()?
                    .contact_group_resource_name.as_deref()?;
                if rn == "contactGroups/myContacts" { return None; }
                group_names.get(rn).cloned()
            }).collect()
        })
        .unwrap_or_default()
}

pub async fn update_phone_numbers<F>(hub: &HubType, person: &google_people1::api::Person, transform: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Option<String>,
{
    let resource_name = get_resource_name(person)?;

    let mut updated = person.clone();
    if let Some(ref mut nums) = updated.phone_numbers {
        for pn in nums.iter_mut() {
            if let Some(ref val) = pn.value
                && let Some(new_val) = transform(val) {
                    pn.value = Some(new_val);
                }
        }
    }
    retry_api(|| {
        let req = hub.people()
            .update_contact(updated.clone(), resource_name)
            .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]));
        async { req.doit().await }
    }).await?;
    eprintln!("  Fixed: {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

pub async fn fetch_all_contact_groups(hub: &HubType) -> Result<Vec<google_people1::api::ContactGroup>, Box<dyn std::error::Error>> {
    let mut all_groups: Vec<google_people1::api::ContactGroup> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let (_response, result) = retry_api(|| {
            let mut req = hub.contact_groups().list();
            if let Some(ref token) = page_token {
                req = req.page_token(token);
            }
            async { req.doit().await }
        }).await?;
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
    let mut http = hyper_util::client::legacy::connect::HttpConnector::new();
    http.set_connect_timeout(Some(std::time::Duration::from_secs(30)));
    Ok(hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http2()
        .wrap_connector(http))
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
        .pool_idle_timeout(Duration::from_secs(30))
        .build(build_connector()?);

    Ok(PeopleService::new(client, auth))
}

// --- Prompt helpers ---

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
    let options = PHONE_LABEL_OPTIONS;
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

fn google_phone_type_to_vcard(ptype: &str) -> &'static str {
    match ptype.to_lowercase().as_str() {
        "mobile" => "CELL",
        "home" => "HOME",
        "work" => "WORK",
        "main" => "VOICE",
        "homefax" | "home fax" => "HOME,FAX",
        "workfax" | "work fax" => "WORK,FAX",
        _ => "VOICE",
    }
}

fn google_email_type_to_vcard(etype: &str) -> &'static str {
    match etype.to_lowercase().as_str() {
        "home" => "HOME",
        "work" => "WORK",
        _ => "OTHER",
    }
}

fn google_address_type_to_vcard(atype: &str) -> &'static str {
    match atype.to_lowercase().as_str() {
        "home" => "HOME",
        "work" => "WORK",
        _ => "OTHER",
    }
}

pub fn person_to_vcard(person: &google_people1::api::Person, uid: &str, rev: &str) -> String {
    let names = person.names.as_ref().and_then(|n| n.first());
    let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
    let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");
    let full = person_display_name(person);
    let nickname = person.nicknames.as_ref()
        .and_then(|n| n.first())
        .and_then(|n| n.value.as_deref())
        .unwrap_or("");

    let mut vcard = String::new();
    vcard.push_str("BEGIN:VCARD\r\n");
    vcard.push_str("VERSION:3.0\r\n");
    vcard.push_str(&format!("UID:{}\r\n", uid));
    vcard.push_str(&format!("REV:{}\r\n", rev));
    vcard.push_str(&format!("FN:{}\r\n", full));
    if !given.is_empty() || !family.is_empty() {
        vcard.push_str(&format!("N:{};{};;;\r\n", family, given));
    }
    if !nickname.is_empty() {
        vcard.push_str(&format!("NICKNAME:{}\r\n", nickname));
    }

    // Organization
    if let Some(org) = person.organizations.as_ref().and_then(|o| o.first()) {
        if let Some(ref name) = org.name {
            vcard.push_str(&format!("ORG:{}\r\n", name));
        }
        if let Some(ref title) = org.title {
            vcard.push_str(&format!("TITLE:{}\r\n", title));
        }
    }

    // Phone numbers
    if let Some(ref phones) = person.phone_numbers {
        for phone in phones {
            if let Some(ref value) = phone.value {
                let ptype = phone.type_.as_deref().or(phone.formatted_type.as_deref()).unwrap_or("voice");
                let vcard_type = google_phone_type_to_vcard(ptype);
                vcard.push_str(&format!("TEL;TYPE={}:{}\r\n", vcard_type, value));
            }
        }
    }

    // Email addresses
    if let Some(ref emails) = person.email_addresses {
        for email in emails {
            if let Some(ref value) = email.value {
                let etype = email.type_.as_deref().or(email.formatted_type.as_deref()).unwrap_or("other");
                let vcard_type = google_email_type_to_vcard(etype);
                vcard.push_str(&format!("EMAIL;TYPE={}:{}\r\n", vcard_type, value));
            }
        }
    }

    // Addresses
    if let Some(ref addresses) = person.addresses {
        for addr in addresses {
            let street = addr.street_address.as_deref().unwrap_or("");
            let city = addr.city.as_deref().unwrap_or("");
            let region = addr.region.as_deref().unwrap_or("");
            let postal = addr.postal_code.as_deref().unwrap_or("");
            let country = addr.country.as_deref().unwrap_or("");
            let atype = addr.type_.as_deref().unwrap_or("other");
            let vcard_type = google_address_type_to_vcard(atype);
            vcard.push_str(&format!("ADR;TYPE={}:;;{};{};{};{};{}\r\n", vcard_type, street, city, region, postal, country));
        }
    }

    // Birthday
    if let Some(ref birthdays) = person.birthdays
        && let Some(bday) = birthdays.first()
        && let Some(ref date) = bday.date
        && let (Some(y), Some(m), Some(d)) = (date.year, date.month, date.day) {
            vcard.push_str(&format!("BDAY:{:04}-{:02}-{:02}\r\n", y, m, d));
        }

    vcard.push_str("END:VCARD\r\n");
    vcard
}
