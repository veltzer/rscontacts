use clap::Parser;
use rscontacts::{Cli, Commands};

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
