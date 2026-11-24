# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Debug build
cargo build --release                # Release build (stripped, LTO)
cargo nextest run                    # Run tests (preferred over cargo test)
cargo nextest run --release          # Run tests in release mode
cargo nextest run -E 'test(test_name)'  # Run a single test
cargo clippy                         # Lint
```

Requires Rust edition 2024. Uses `build.rs` to embed git metadata (SHA, branch, describe, dirty) and rustc version at compile time.

## Architecture

Single-binary Rust CLI for auditing and fixing Google Contacts. Three source files:

- **`src/main.rs`** - CLI definition using `clap` derive. Defines `Commands` enum and dispatches to `commands::*`. All unit tests live in `mod tests` here.
- **`src/commands.rs`** - Command implementations. Each `cmd_*` function handles one subcommand. `cmd_check_all` must call every individual check command.
- **`src/helpers.rs`** - Shared utilities: Google API client setup (`build_hub`, `fetch_all_contacts`, `fetch_all_contact_groups`), contact field helpers (`person_display_name`, `person_email`, `phone_has_type`, `has_user_label`), phone/name/email validation (`is_fixable_phone`, `is_english_name`, `is_all_caps`, `is_valid_email`, `has_country_code`, `is_correct_phone_format`, `fix_phone_format`), and check helpers (`check_name_issues`, `check_phone_issues`, etc.).

## Key Conventions

- Every new check command must be added to `check-all` (`cmd_check_all`).
- Check helper functions take a `CheckContext` struct (containing `fix`, `dry_run`, `prefix`, `header`, `quiet`, `user_groups`, `label_names`, `group_names`) so they can be reused by both standalone commands and `check-all`.
- "Labels" in Google Contacts = contact group memberships on Person (checked by `check-contact-no-label`). Phone "labels" (mobile/home/work) = `type_`/`formatted_type` on PhoneNumber (checked by `check-phone-no-label`). These are distinct concepts.
- Phone format checks use `is_fixable_phone()` to skip non-numeric entries (star codes, short codes, alphanumeric strings).
- OAuth credentials and the token cache come from pass(1) first (`gcp/rscontacts/oauth-client`, `gcp/rscontacts/token`, overridable in the `[pass]` table of config.toml) and fall back to `~/.config/rscontacts/credentials.json` and `token_cache.json` when `pass` or the entry is missing (`src/pass.rs`). Never write refreshed access tokens to pass (git-tracked store): only a new authorization or a changed refresh token. Never log secret values.
- GCP project: `rscontacts`. The repo's `.gcp.conf` names the gcloud configuration `rscontacts`, which bash-bashy's `prompt_gcp` activates on `cd` into the repo.
- Most check commands support `--fix` and `--dry-run` flags. A 500ms delay (`MUTATE_DELAY`) is applied between mutations to avoid rate limiting.
- All Google API calls use `retry_api()` to handle transient HTTP errors (429, 502, 503, 504) with exponential backoff. See the [Transport Errors](docs/src/transport-errors.md) book chapter for details.
