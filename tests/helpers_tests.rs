use rscontacts::helpers::*;
use rscontacts::helpers::Config;
use google_people1::api::*;

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

#[test]
fn test_person_to_vcard_basic() {
    let person = Person {
        names: Some(vec![Name {
            given_name: Some("John".to_string()),
            family_name: Some("Doe".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let vcard = person_to_vcard(&person, "test-uid", "20260310T120000Z");
    assert!(vcard.contains("BEGIN:VCARD\r\n"));
    assert!(vcard.contains("VERSION:3.0\r\n"));
    assert!(vcard.contains("UID:test-uid\r\n"));
    assert!(vcard.contains("FN:John Doe\r\n"));
    assert!(vcard.contains("N:Doe;John;;;\r\n"));
    assert!(vcard.contains("END:VCARD\r\n"));
}

#[test]
fn test_person_to_vcard_with_phone() {
    let person = Person {
        names: Some(vec![Name {
            given_name: Some("Jane".to_string()),
            ..Default::default()
        }]),
        phone_numbers: Some(vec![PhoneNumber {
            value: Some("+972-501234567".to_string()),
            type_: Some("mobile".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let vcard = person_to_vcard(&person, "uid-2", "20260310T120000Z");
    assert!(vcard.contains("TEL;TYPE=CELL:+972-501234567\r\n"));
}

#[test]
fn test_person_to_vcard_with_email() {
    let person = Person {
        names: Some(vec![Name {
            given_name: Some("Bob".to_string()),
            ..Default::default()
        }]),
        email_addresses: Some(vec![EmailAddress {
            value: Some("bob@example.com".to_string()),
            type_: Some("work".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let vcard = person_to_vcard(&person, "uid-3", "20260310T120000Z");
    assert!(vcard.contains("EMAIL;TYPE=WORK:bob@example.com\r\n"));
}

#[test]
fn test_person_to_vcard_with_org() {
    let person = Person {
        names: Some(vec![Name {
            given_name: Some("Alice".to_string()),
            ..Default::default()
        }]),
        organizations: Some(vec![Organization {
            name: Some("Acme Corp".to_string()),
            title: Some("Engineer".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let vcard = person_to_vcard(&person, "uid-4", "20260310T120000Z");
    assert!(vcard.contains("ORG:Acme Corp\r\n"));
    assert!(vcard.contains("TITLE:Engineer\r\n"));
}

#[test]
fn test_person_to_vcard_with_birthday() {
    let person = Person {
        names: Some(vec![Name {
            given_name: Some("Eve".to_string()),
            ..Default::default()
        }]),
        birthdays: Some(vec![Birthday {
            date: Some(Date {
                year: Some(1990),
                month: Some(5),
                day: Some(15),
            }),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let vcard = person_to_vcard(&person, "uid-5", "20260310T120000Z");
    assert!(vcard.contains("BDAY:1990-05-15\r\n"));
}

#[test]
fn test_person_to_vcard_with_address() {
    let person = Person {
        names: Some(vec![Name {
            given_name: Some("Charlie".to_string()),
            ..Default::default()
        }]),
        addresses: Some(vec![Address {
            street_address: Some("123 Main St".to_string()),
            city: Some("Springfield".to_string()),
            region: Some("IL".to_string()),
            postal_code: Some("62701".to_string()),
            country: Some("US".to_string()),
            type_: Some("home".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let vcard = person_to_vcard(&person, "uid-6", "20260310T120000Z");
    assert!(vcard.contains("ADR;TYPE=HOME:;;123 Main St;Springfield;IL;62701;US\r\n"));
}

#[test]
fn test_person_to_vcard_no_name_uses_fallback() {
    let person = Person::default();
    let vcard = person_to_vcard(&person, "uid-7", "20260310T120000Z");
    assert!(vcard.contains("FN:<no name>\r\n"));
    assert!(!vcard.contains("\r\nN:"));
}

#[test]
fn test_person_to_vcard_with_nickname() {
    let person = Person {
        names: Some(vec![Name {
            given_name: Some("Robert".to_string()),
            ..Default::default()
        }]),
        nicknames: Some(vec![Nickname {
            value: Some("Bob".to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let vcard = person_to_vcard(&person, "uid-8", "20260310T120000Z");
    assert!(vcard.contains("NICKNAME:Bob\r\n"));
}

#[test]
fn test_config_parse_empty() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.check_all.skip.is_empty());
    assert!(config.check_contact_given_name_regexp.allow.is_none());
}

#[test]
fn test_config_parse_skip_list() {
    let config: Config = toml::from_str(r#"
[check-all]
skip = ["check-phone-format", "check-contact-email"]
"#).unwrap();
    assert_eq!(config.check_all.skip.len(), 2);
    assert_eq!(config.check_all.skip[0], "check-phone-format");
}

#[test]
fn test_config_parse_name_regexp() {
    let config: Config = toml::from_str(r#"
[check-contact-given-name-regexp]
allow = '^[A-Z][a-z]*$'
"#).unwrap();
    assert_eq!(config.check_contact_given_name_regexp.allow.as_deref(), Some("^[A-Z][a-z]*$"));
}

#[test]
fn test_config_parse_companies() {
    let config: Config = toml::from_str(r#"
[check-contact-name-is-company]
companies = ["Acme", "Globex"]
"#).unwrap();
    assert_eq!(config.check_contact_name_is_company.companies, vec!["Acme", "Globex"]);
}

#[test]
fn test_config_parse_given_names() {
    let config: Config = toml::from_str(r#"
[check-contact-given-name-known]
names = ["John", "Jane"]
"#).unwrap();
    assert_eq!(config.check_contact_given_name_known.names, vec!["John", "Jane"]);
}

#[test]
fn test_config_parse_unknown_sections_ignored() {
    let config: Config = toml::from_str(r#"
[unknown-section]
key = "value"

[check-all]
skip = ["a"]
"#).unwrap();
    assert_eq!(config.check_all.skip, vec!["a"]);
}

#[test]
fn test_config_skip_set_behavior() {
    let config: Config = toml::from_str(r#"
[check-all]
skip = ["check-phone-format", "check-contact-email"]
"#).unwrap();
    let skip: std::collections::HashSet<&str> = config.check_all.skip.iter().map(|s| s.as_str()).collect();
    assert!(skip.contains("check-phone-format"));
    assert!(skip.contains("check-contact-email"));
    assert!(!skip.contains("check-phone-countrycode"));
    assert!(!skip.contains("check-contact-given-name-known"));
}

#[test]
fn test_config_skip_empty_means_all_run() {
    let config: Config = toml::from_str(r#"
[check-all]
skip = []
"#).unwrap();
    let skip: std::collections::HashSet<&str> = config.check_all.skip.iter().map(|s| s.as_str()).collect();
    assert!(!skip.contains("check-phone-format"));
    assert!(!skip.contains("check-contact-given-name-known"));
}

#[test]
fn test_config_default_skip_is_empty() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.check_all.skip.is_empty());
}

#[test]
fn test_config_multiple_regexp_sections() {
    let config: Config = toml::from_str(r#"
[check-contact-given-name-regexp]
allow = '^[A-Z]'

[check-contact-family-name-regexp]
allow = '^[A-Z][a-z]+'

[check-contact-suffix-regexp]
allow = '^[1-9]\d*$'

[check-contact-label-regexp]
allow = '^(type|company):.*$'
"#).unwrap();
    assert_eq!(config.check_contact_given_name_regexp.allow.as_deref(), Some("^[A-Z]"));
    assert_eq!(config.check_contact_family_name_regexp.allow.as_deref(), Some("^[A-Z][a-z]+"));
    assert_eq!(config.check_contact_suffix_regexp.allow.as_deref(), Some("^[1-9]\\d*$"));
    assert_eq!(config.check_contact_label_regexp.allow.as_deref(), Some("^(type|company):.*$"));
}
