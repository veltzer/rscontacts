use rscontacts::helpers::*;

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
