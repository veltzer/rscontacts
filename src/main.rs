mod commands;
mod helpers;

use clap::{CommandFactory, Parser, Subcommand};

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
        /// Also show contact labels (contact group memberships)
        #[arg(long)]
        labels: bool,
        /// Only show starred contacts
        #[arg(long)]
        starred: bool,
    },
    /// Print contacts with non-English names
    CheckContactNameEnglish {
        /// Interactively fix each non-English contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts with all-caps names
    CheckContactNameCaps {
        /// Interactively fix each all-caps contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts whose name doesn't start with a capital letter
    CheckContactNameFirstCapitalLetter {
        /// Interactively fix each contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check first names against allow regex defined in config.toml
    CheckContactFirstnameRegexp {
        /// Interactively fix each flagged contact (swap/rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check last names against allow regex defined in config.toml
    CheckContactLastnameRegexp {
        /// Interactively fix each flagged contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts with reversed name order (e.g. "Family, Given")
    CheckContactNameOrder {
        /// Interactively fix each contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts that share the same display name
    CheckContactDisplaynameDuplicate {
        /// Interactively fix each duplicate (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that same-name contacts are distinguished with sequential numeric suffixes (1, 2, ...)
    CheckContactSamenameSuffix {
        /// Automatically assign sequential suffixes
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
        /// Show each check name as it runs
        #[arg(short, long)]
        verbose: bool,
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
    CheckContactNoLabel {
        /// Interactively fix: delete contact or assign a label
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts with phone numbers missing a label (mobile/home/work/etc)
    CheckPhoneNoLabel {
        /// Interactively fix phones without labels
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
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
    CheckContactEmail,
    /// Print contacts with uppercase letters in email addresses
    CheckContactEmailCaps {
        /// Automatically lowercase emails
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts that have the same email address attached twice
    CheckContactEmailDuplicate {
        /// Interactively remove duplicate email addresses
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print contacts that have the same phone number attached twice
    CheckPhoneDuplicate {
        /// Interactively remove duplicate phone numbers
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Print labels (contact groups) that have no contacts
    CheckContactLabelNophone {
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
    /// Print contact labels (groups) that are not all lowercase (camelCase or uppercase)
    CheckContactLabelCamelcase {
        /// Rename labels to lowercase
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check for contacts with numeric surnames (e.g. "1", "2") and move them to the suffix field
    CheckContactNameNumericSurname {
        /// Move numeric surnames to the suffix field
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Review all phones with a specific label (e.g. "Work Fax")
    ReviewPhoneLabel {
        /// The phone label to review (case-insensitive)
        label: String,
        /// Interactively fix each phone (delete/relabel/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove a contact label (group) from all contacts that have it
    RemoveLabelFromAllContacts {
        /// The label name to remove (case-insensitive)
        label: String,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default CryptoProvider");

    let cli = Cli::parse();

    match cli.command {
        Commands::Auth { no_browser, force } => commands::cmd_auth(no_browser, force).await?,
        Commands::List { emails, labels, starred } => commands::cmd_list(emails, labels, starred).await?,
        Commands::CheckContactNameEnglish { fix, dry_run } => commands::cmd_check_contact_name_english(fix, dry_run).await?,
        Commands::CheckContactNameCaps { fix, dry_run } => commands::cmd_check_contact_name_caps(fix, dry_run).await?,
        Commands::CheckContactNameFirstCapitalLetter { fix, dry_run } => commands::cmd_check_contact_name_first_capital_letter(fix, dry_run).await?,
        Commands::CheckContactFirstnameRegexp { fix, dry_run } => commands::cmd_check_contact_firstname_regexp(fix, dry_run).await?,
        Commands::CheckContactLastnameRegexp { fix, dry_run } => commands::cmd_check_contact_lastname_regexp(fix, dry_run).await?,
        Commands::CheckContactNameOrder { fix, dry_run } => commands::cmd_check_contact_name_order(fix, dry_run).await?,
        Commands::CheckContactDisplaynameDuplicate { fix, dry_run } => commands::cmd_check_contact_displayname_duplicate(fix, dry_run).await?,
        Commands::CheckContactSamenameSuffix { fix, dry_run } => commands::cmd_check_contact_samename_suffix(fix, dry_run).await?,
        Commands::CheckPhoneCountrycode { fix, dry_run, ref country } => commands::cmd_check_phone_countrycode(fix, dry_run, country).await?,
        Commands::CheckPhoneFormat { fix, dry_run, ref country } => commands::cmd_check_phone_format(fix, dry_run, country).await?,
        Commands::CheckContactNoLabel { fix, dry_run } => commands::cmd_check_contact_no_label(fix, dry_run).await?,
        Commands::CheckPhoneNoLabel { fix, dry_run } => commands::cmd_check_phone_no_label(fix, dry_run).await?,
        Commands::CheckPhoneLabelEnglish { fix, dry_run } => commands::cmd_check_phone_label_english(fix, dry_run).await?,
        Commands::CheckContactEmail => commands::cmd_check_contact_email().await?,
        Commands::CheckContactEmailCaps { fix, dry_run } => commands::cmd_check_contact_email_caps(fix, dry_run).await?,
        Commands::CheckContactEmailDuplicate { fix, dry_run } => commands::cmd_check_contact_email_duplicate(fix, dry_run).await?,
        Commands::CheckPhoneDuplicate { fix, dry_run } => commands::cmd_check_phone_duplicate(fix, dry_run).await?,
        Commands::CheckContactLabelNophone { fix, dry_run } => commands::cmd_check_contact_label_nophone(fix, dry_run).await?,
        Commands::CheckContactLabelSpace { fix, dry_run } => commands::cmd_check_contact_label_space(fix, dry_run).await?,
        Commands::CheckContactLabelCamelcase { fix, dry_run } => commands::cmd_check_contact_label_camelcase(fix, dry_run).await?,
        Commands::CheckContactNameNumericSurname { fix, dry_run } => commands::cmd_check_contact_name_numeric_surname(fix, dry_run).await?,
        Commands::ShowContact { ref name } => commands::cmd_show_contact(name).await?,
        Commands::RemoveLabelFromAllContacts { ref label, dry_run } => commands::cmd_remove_label_from_all_contacts(label, dry_run).await?,
        Commands::ReviewPhoneLabel { ref label, fix, dry_run } => commands::cmd_review_phone_label(label, fix, dry_run).await?,
        Commands::ShowPhoneLabels => commands::cmd_show_phone_labels().await?,
        Commands::ShowContactLabels => commands::cmd_show_contact_labels().await?,
        Commands::CheckAll { fix, dry_run, stats, verbose, ref country } => commands::cmd_check_all(fix, dry_run, stats, verbose, country).await?,
        Commands::Version => {
            println!("rscontacts {} by {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"));
            println!("GIT_DESCRIBE: {}", env!("GIT_DESCRIBE"));
            println!("GIT_SHA: {}", env!("GIT_SHA"));
            println!("GIT_BRANCH: {}", env!("GIT_BRANCH"));
            println!("GIT_DIRTY: {}", env!("GIT_DIRTY"));
            println!("RUSTC_SEMVER: {}", env!("RUSTC_SEMVER"));
            println!("RUST_EDITION: {}", env!("RUST_EDITION"));
            println!("BUILD_TIMESTAMP: {}", env!("BUILD_TIMESTAMP"));
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
    use crate::helpers::*;
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
        let cli = Cli::parse_from(["rscontacts", "check-contact-name-english"]);
        assert!(matches!(cli.command, Commands::CheckContactNameEnglish { fix: false, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_english_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-contact-name-english", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckContactNameEnglish { fix: true, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_english_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "check-contact-name-english", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::CheckContactNameEnglish { fix: true, dry_run: true }));
    }

    #[test]
    fn test_cli_check_name_caps_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "check-contact-name-caps"]);
        assert!(matches!(cli.command, Commands::CheckContactNameCaps { fix: false, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_caps_fix() {
        let cli = Cli::parse_from(["rscontacts", "check-contact-name-caps", "--fix"]);
        assert!(matches!(cli.command, Commands::CheckContactNameCaps { fix: true, dry_run: false }));
    }

    #[test]
    fn test_cli_check_name_caps_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "check-contact-name-caps", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::CheckContactNameCaps { fix: true, dry_run: true }));
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
    fn test_is_numeric_string() {
        assert!(is_numeric_string("1"));
        assert!(is_numeric_string("123"));
        assert!(!is_numeric_string(""));
        assert!(!is_numeric_string("abc"));
        assert!(!is_numeric_string("1a"));
    }

    #[test]
    fn test_split_name_suffix() {
        assert_eq!(split_name_suffix("Mike 1"), ("Mike", Some(1)));
        assert_eq!(split_name_suffix("Mike 2"), ("Mike", Some(2)));
        assert_eq!(split_name_suffix("Mike"), ("Mike", None));
        assert_eq!(split_name_suffix("John Doe"), ("John Doe", None));
        assert_eq!(split_name_suffix("Mike 10"), ("Mike", Some(10)));
        assert_eq!(split_name_suffix("Jean-Pierre 3"), ("Jean-Pierre", Some(3)));
    }

    #[test]
    fn test_token_cache_path_is_in_config_dir() {
        let path = token_cache_path();
        assert!(path.ends_with("token_cache.json"));
        assert!(path.to_str().unwrap().contains(".config/rscontacts"));
    }
}
