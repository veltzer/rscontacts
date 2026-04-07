use clap::Parser;
use rscontacts::{Cli, Commands, commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default CryptoProvider");

    let cli = Cli::parse();
    rscontacts::helpers::set_transport_errors(cli.transport_errors);

    match cli.command {
        Commands::AllChecks { fix, dry_run, stats, verbose, ref country } => commands::cmd_check_all(fix, dry_run, stats, verbose, country).await?,
        Commands::Auth { no_browser, force } => commands::cmd_auth(no_browser, force).await?,
        Commands::CheckContactCompanyExists { fix, dry_run } => commands::cmd_check_contact_company_exists(fix, dry_run).await?,
        Commands::CheckContactCompanyKnown { fix, dry_run } => commands::cmd_check_contact_company_known(fix, dry_run).await?,
        Commands::CheckContactDisplaynameDuplicate { fix, dry_run } => commands::cmd_check_contact_displayname_duplicate(fix, dry_run).await?,
        Commands::CheckContactNoDisplayname { fix, dry_run } => commands::cmd_check_contact_no_displayname(fix, dry_run).await?,
        Commands::CheckContactEmail { fix, dry_run } => commands::cmd_check_contact_email(fix, dry_run).await?,
        Commands::CheckContactEmailDuplicate { fix, dry_run } => commands::cmd_check_contact_email_duplicate(fix, dry_run).await?,
        Commands::CheckContactFamilyNameRegexp { fix, dry_run } => commands::cmd_check_contact_family_name_regexp(fix, dry_run).await?,
        Commands::CheckContactGivenNameExists { fix, dry_run } => commands::cmd_check_contact_given_name_exists(fix, dry_run).await?,
        Commands::CheckContactGivenNameKnown { fix, dry_run } => commands::cmd_check_contact_given_name_known(fix, dry_run).await?,
        Commands::CheckContactGivenNameRegexp { fix, dry_run } => commands::cmd_check_contact_given_name_regexp(fix, dry_run).await?,
        Commands::CheckContactLabelNophone { fix, dry_run } => commands::cmd_check_contact_label_nophone(fix, dry_run).await?,
        Commands::CheckContactLabelRegexp { fix, dry_run } => commands::cmd_check_contact_label_regexp(fix, dry_run).await?,
        Commands::CheckContactNoGivenName { fix, dry_run } => commands::cmd_check_contact_no_given_name(fix, dry_run).await?,
        Commands::CheckContactNoIdentity { fix, dry_run } => commands::cmd_check_contact_no_identity(fix, dry_run).await?,
        Commands::CheckContactNoLabel { fix, dry_run } => commands::cmd_check_contact_no_label(fix, dry_run).await?,
        Commands::CheckContactNoMiddleName { fix, dry_run } => commands::cmd_check_contact_no_middle_name(fix, dry_run).await?,
        Commands::CheckContactNoNickname { fix, dry_run } => commands::cmd_check_contact_no_nickname(fix, dry_run).await?,
        Commands::CheckContactSuffixRegexp { fix, dry_run } => commands::cmd_check_contact_suffix_regexp(fix, dry_run).await?,
        Commands::CheckContactTypeCompanyGivenName { fix, auto_fix, dry_run } => commands::cmd_check_contact_type_company_given_name(fix, auto_fix, dry_run).await?,
        Commands::CheckContactTypeCompanyNoLabel { fix, auto_fix, dry_run } => commands::cmd_check_contact_type_company_no_label(fix, auto_fix, dry_run).await?,
        Commands::CheckContactTypeCompanyNoCompany { fix, dry_run } => commands::cmd_check_contact_type_company_no_company(fix, dry_run).await?,
        Commands::CheckContactType { fix, dry_run } => commands::cmd_check_contact_type(fix, dry_run).await?,
        Commands::CheckPhoneCountryLabel { fix, dry_run } => commands::cmd_check_phone_country_label(fix, dry_run).await?,
        Commands::CheckPhoneCountrycode { fix, dry_run, ref country } => commands::cmd_check_phone_countrycode(fix, dry_run, country).await?,
        Commands::CheckPhoneDuplicate { fix, dry_run } => commands::cmd_check_phone_duplicate(fix, dry_run).await?,
        Commands::CheckPhoneFormat { fix, dry_run, ref country } => commands::cmd_check_phone_format(fix, dry_run, country).await?,
        Commands::CheckPhoneLabelEnglish { fix, dry_run } => commands::cmd_check_phone_label_english(fix, dry_run).await?,
        Commands::CheckPhoneLabelMissing { fix, dry_run } => commands::cmd_check_phone_label_missing(fix, dry_run).await?,
        Commands::MergeByEmail { fix, dry_run } => commands::cmd_merge_by_email(fix, dry_run).await?,
        Commands::MergeByPhone { fix, dry_run } => commands::cmd_merge_by_phone(fix, dry_run).await?,
        Commands::MoveFamilyToSuffix { dry_run } => commands::cmd_move_family_to_suffix(dry_run).await?,
        Commands::MoveSuffixToFamily { dry_run } => commands::cmd_move_suffix_to_family(dry_run).await?,
        Commands::CompactSuffixesForContacts { dry_run } => commands::cmd_compact_suffixes_for_contacts(dry_run).await?,
        Commands::Complete { shell } => rscontacts::generate_completions(shell),
        Commands::EditContact { ref name } => commands::cmd_edit_contact(name).await?,
        Commands::ExportJson { short } => commands::cmd_export_json(short).await?,
        Commands::InitConfig { force } => commands::cmd_init_config(force)?,
        Commands::List { emails, labels, starred } => commands::cmd_list(emails, labels, starred).await?,
        Commands::SyncGnomeContacts { dry_run } => commands::cmd_sync_gnome_contacts(dry_run).await?,
        Commands::RemoveLabelFromAllContacts { ref label, dry_run } => commands::cmd_remove_label_from_all_contacts(label, dry_run).await?,
        Commands::ReviewEmailLabel { ref label, fix, dry_run } => commands::cmd_review_email_label(label, fix, dry_run).await?,
        Commands::ReviewPhoneLabel { ref label, fix, dry_run } => commands::cmd_review_phone_label(label, fix, dry_run).await?,
        Commands::ShowContact { ref name } => commands::cmd_show_contact(name).await?,
        Commands::ShowContactLabels => commands::cmd_show_contact_labels().await?,
        Commands::ShowEmailLabels => commands::cmd_show_email_labels().await?,
        Commands::ShowPhoneLabels => commands::cmd_show_phone_labels().await?,
        Commands::TestConnect => commands::cmd_test_connect().await?,
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
    }

    Ok(())
}
