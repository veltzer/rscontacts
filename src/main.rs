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
    /// Check given names against allow regex defined in config.toml
    CheckContactGivenNameRegexp {
        /// Interactively fix each flagged contact (swap/rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check suffixes against allow regex (default: numeric)
    CheckContactSuffixRegexp {
        /// Interactively fix each flagged contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check family names against allow regex defined in config.toml
    CheckContactFamilyNameRegexp {
        /// Interactively fix each flagged contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check contacts that have no given name but have a family name
    CheckContactNoGivenName {
        /// Interactively fix each flagged contact
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check contacts that have no type tag (type:Person or type:Company)
    CheckContactNoIdentity {
        /// Interactively fix each flagged contact
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that all given names are in the allowed list from config
    CheckContactGivenNameKnown {
        /// Interactively fix each flagged contact
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that all company fields are in the known companies list from config
    CheckContactCompanyKnown {
        /// Interactively fix each flagged contact
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that every company in the config has at least one contact
    CheckContactCompanyExists {
        /// Interactively fix each flagged contact
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
    /// Run all checks
    #[command(name = "all-checks")]
    AllChecks {
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
    CheckPhoneLabelMissing {
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
    /// Print contacts with invalid or uppercase email addresses
    CheckContactEmail {
        /// Automatically lowercase emails with uppercase letters
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
    /// Check that every contact has exactly one of type:Person or type:Company labels
    CheckContactType {
        /// Interactively fix each flagged contact
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that no contact has a middle name set
    CheckContactNoMiddleName {
        /// Interactively fix each flagged contact
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that no contact has a nickname set
    CheckContactNoNickname {
        /// Interactively fix each flagged contact
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
    /// Check contact labels (groups) against allow regex defined in config.toml
    CheckContactLabelRegexp {
        /// Interactively rename labels that don't match
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
    /// Compact suffixes for contacts sharing the same base name (given + family)
    CompactSuffixesForContacts {
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Show all details about a specific contact
    ShowContact {
        /// Name (or part of name) to search for
        name: String,
    },
    /// Move a given name to the company field for all contacts with that given name
    MoveGivenNameToCompany {
        /// The given name to move to company
        name: String,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Auto-assign type:Person/type:Company to contacts missing a type label
    AutoContactType {
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Rename labels of company contacts to "Company:[label]"
    CompanyLabels {
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Interactively edit a contact
    EditContact {
        /// Name (or part of name) to search for
        name: String,
    },
    /// Generate a default config file at ~/.config/rscontacts/config.toml
    InitConfig {
        /// Overwrite existing config file
        #[arg(long)]
        force: bool,
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
        Commands::CheckContactGivenNameRegexp { fix, dry_run } => commands::cmd_check_contact_given_name_regexp(fix, dry_run).await?,
        Commands::CheckContactSuffixRegexp { fix, dry_run } => commands::cmd_check_contact_suffix_regexp(fix, dry_run).await?,
        Commands::CheckContactFamilyNameRegexp { fix, dry_run } => commands::cmd_check_contact_family_name_regexp(fix, dry_run).await?,
        Commands::CheckContactNoGivenName { fix, dry_run } => commands::cmd_check_contact_no_given_name(fix, dry_run).await?,
        Commands::CheckContactNoIdentity { fix, dry_run } => commands::cmd_check_contact_no_identity(fix, dry_run).await?,
        Commands::CheckContactGivenNameKnown { fix, dry_run } => commands::cmd_check_contact_given_name_known(fix, dry_run).await?,
        Commands::CheckContactCompanyKnown { fix, dry_run } => commands::cmd_check_contact_company_known(fix, dry_run).await?,
        Commands::CheckContactCompanyExists { fix, dry_run } => commands::cmd_check_contact_company_exists(fix, dry_run).await?,
        Commands::CheckContactDisplaynameDuplicate { fix, dry_run } => commands::cmd_check_contact_displayname_duplicate(fix, dry_run).await?,
        Commands::CheckPhoneCountrycode { fix, dry_run, ref country } => commands::cmd_check_phone_countrycode(fix, dry_run, country).await?,
        Commands::CheckPhoneFormat { fix, dry_run, ref country } => commands::cmd_check_phone_format(fix, dry_run, country).await?,
        Commands::CheckContactNoLabel { fix, dry_run } => commands::cmd_check_contact_no_label(fix, dry_run).await?,
        Commands::CheckPhoneLabelMissing { fix, dry_run } => commands::cmd_check_phone_label_missing(fix, dry_run).await?,
        Commands::CheckPhoneLabelEnglish { fix, dry_run } => commands::cmd_check_phone_label_english(fix, dry_run).await?,
        Commands::CheckContactEmail { fix, dry_run } => commands::cmd_check_contact_email(fix, dry_run).await?,
        Commands::CheckContactEmailDuplicate { fix, dry_run } => commands::cmd_check_contact_email_duplicate(fix, dry_run).await?,
        Commands::CheckPhoneDuplicate { fix, dry_run } => commands::cmd_check_phone_duplicate(fix, dry_run).await?,
        Commands::CheckContactType { fix, dry_run } => commands::cmd_check_contact_type(fix, dry_run).await?,
        Commands::CheckContactNoMiddleName { fix, dry_run } => commands::cmd_check_contact_no_middle_name(fix, dry_run).await?,
        Commands::CheckContactNoNickname { fix, dry_run } => commands::cmd_check_contact_no_nickname(fix, dry_run).await?,
        Commands::CheckContactLabelNophone { fix, dry_run } => commands::cmd_check_contact_label_nophone(fix, dry_run).await?,
        Commands::CheckContactLabelRegexp { fix, dry_run } => commands::cmd_check_contact_label_regexp(fix, dry_run).await?,
        Commands::CompactSuffixesForContacts { dry_run } => commands::cmd_compact_suffixes_for_contacts(dry_run).await?,
        Commands::ShowContact { ref name } => commands::cmd_show_contact(name).await?,
        Commands::MoveGivenNameToCompany { ref name, dry_run } => commands::cmd_move_given_name_to_company(name, dry_run).await?,
        Commands::AutoContactType { dry_run } => commands::cmd_auto_contact_type(dry_run).await?,
        Commands::CompanyLabels { dry_run } => commands::cmd_company_labels(dry_run).await?,
        Commands::EditContact { ref name } => commands::cmd_edit_contact(name).await?,
        Commands::RemoveLabelFromAllContacts { ref label, dry_run } => commands::cmd_remove_label_from_all_contacts(label, dry_run).await?,
        Commands::ReviewPhoneLabel { ref label, fix, dry_run } => commands::cmd_review_phone_label(label, fix, dry_run).await?,
        Commands::ShowPhoneLabels => commands::cmd_show_phone_labels().await?,
        Commands::ShowContactLabels => commands::cmd_show_contact_labels().await?,
        Commands::AllChecks { fix, dry_run, stats, verbose, ref country } => commands::cmd_check_all(fix, dry_run, stats, verbose, country).await?,
        Commands::InitConfig { force } => commands::cmd_init_config(force)?,
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
    fn test_cli_check_all_subcommand() {
        let cli = Cli::parse_from(["rscontacts", "all-checks"]);
        assert!(matches!(cli.command, Commands::AllChecks { fix: false, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_all_fix() {
        let cli = Cli::parse_from(["rscontacts", "all-checks", "--fix"]);
        assert!(matches!(cli.command, Commands::AllChecks { fix: true, dry_run: false, .. }));
    }

    #[test]
    fn test_cli_check_all_dry_run() {
        let cli = Cli::parse_from(["rscontacts", "all-checks", "--fix", "--dry-run"]);
        assert!(matches!(cli.command, Commands::AllChecks { fix: true, dry_run: true, .. }));
    }

    #[test]
    fn test_cli_check_all_custom_country() {
        let cli = Cli::parse_from(["rscontacts", "all-checks", "--country", "1"]);
        if let Commands::AllChecks { country, .. } = cli.command {
            assert_eq!(country, "1");
        } else {
            panic!("wrong command");
        }
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
