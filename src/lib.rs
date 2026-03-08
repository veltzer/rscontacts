pub mod commands;
pub mod helpers;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rscontacts")]
#[command(about = "Google Contacts CLI tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
    /// Authenticate with Google (opens browser for OAuth2 consent)
    Auth {
        /// Don't open browser automatically; print URL instead
        #[arg(long)]
        no_browser: bool,
        /// Force re-authentication even if a token is already cached
        #[arg(long)]
        force: bool,
    },
    /// Check that all company fields are in the known companies list from config
    CheckContactCompanyExists {
        /// Interactively fix each flagged contact
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that every company in the config has at least one contact
    CheckContactCompanyKnown {
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
    /// Check family names against allow regex defined in config.toml
    CheckContactFamilyNameRegexp {
        /// Interactively fix each flagged contact (rename/delete/skip)
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that every given name in the config has at least one contact
    CheckContactGivenNameExists {
        /// Interactively fix each flagged entry
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
    /// Check given names against allow regex defined in config.toml
    CheckContactGivenNameRegexp {
        /// Interactively fix each flagged contact (swap/rename/delete/skip)
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
    /// Print contacts not assigned to any label (contact group)
    CheckContactNoLabel {
        /// Interactively fix: delete contact or assign a label
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
    /// Check suffixes against allow regex (default: numeric)
    CheckContactSuffixRegexp {
        /// Interactively fix each flagged contact (rename/delete/skip)
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
    /// Print contacts that have the same phone number attached twice
    CheckPhoneDuplicate {
        /// Interactively remove duplicate phone numbers
        #[arg(long)]
        fix: bool,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
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
    /// Print contacts with non-English phone labels
    CheckPhoneLabelEnglish {
        /// Interactively fix non-English phone labels
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
    /// Compact suffixes for contacts sharing the same base name (given + family)
    CompactSuffixesForContacts {
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate shell completions
    Complete {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
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
    /// Remove a contact label (group) from all contacts that have it
    RemoveLabelFromAllContacts {
        /// The label name to remove (case-insensitive)
        label: String,
        /// Show what would be changed without modifying anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Review all emails with a specific label (e.g. "Work")
    ReviewEmailLabel {
        /// The email label to review (case-insensitive)
        label: String,
        /// Interactively fix each email (delete/relabel/skip)
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
    /// Show all details about a specific contact
    ShowContact {
        /// Name (or part of name) to search for
        name: String,
    },
    /// Show all contact labels (contact groups) in use
    ShowContactLabels,
    /// Show all distinct email labels in use
    ShowEmailLabels,
    /// Show all distinct phone labels in use
    ShowPhoneLabels,
    /// Print version information
    Version,
}

pub fn generate_completions(shell: clap_complete::Shell) {
    clap_complete::generate(shell, &mut Cli::command(), "rscontacts", &mut std::io::stdout());
}
