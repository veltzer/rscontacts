# rscontacts - Google Contacts CLI Tool

A command-line tool for managing and auditing Google Contacts, written in Rust.

## Features

- **List and inspect** contacts with flexible display options
- **Audit contacts** with 16+ automated checks for data quality issues
- **Fix issues** interactively with `--fix` support on most checks
- **Dry-run mode** to preview changes before applying them
- **Stats mode** for a quick summary of all issues
- **Shell completions** for bash, zsh, fish, and more

## Checks Available

rscontacts can detect and fix:

- Non-English names, all-caps names, names not starting with a capital letter
- Reversed name order (e.g., "Veltzer, Mark" instead of "Mark Veltzer")
- Phone numbers missing country codes or not in `+CC-NUMBER` format
- Phone numbers without labels (mobile/home/work)
- Non-English phone labels
- Invalid email addresses and emails with uppercase letters
- Duplicate phone numbers and emails on the same contact
- Contacts not assigned to any contact group (label)
- Empty contact groups (labels with no members)
- Contact group names containing spaces

## Technology

- Built with Rust using the [google-people1](https://crates.io/crates/google-people1) crate
- OAuth2 authentication via [yup-oauth2](https://crates.io/crates/yup-oauth2)
- CLI powered by [clap](https://crates.io/crates/clap)
