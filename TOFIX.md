# TOFIX

Findings from a code scan on 2026-10-04.

## High

- `src/helpers.rs:567` - when a number already carries `+`/`00` but `detect_country_code` does not recognise the code, `fix_phone_format` prepends the default country to the full digit string, so a valid `+421 2 1234 5678` (Slovakia) is "fixed" to `+972-421212345678`. `COUNTRY_CODES` (`src/helpers.rs:274`) is missing real codes such as 421, 383, 237, 261, 227, 228 and 211, so this triggers for real contacts. Complete the code table and never inject the default country into a number that already has an explicit country prefix - leave it unfixed instead.

## Medium

- `README.md:88` - the "Processor commands" table lists `auto-contact-type`, `company-labels`, `fix-labels` and `move-given-name-to-company`, none of which exist in the CLI (`rscontacts --help`); remove them, and add the real commands the README omits (`merge-by-email`, `merge-by-phone`, `move-family-to-suffix`, `move-suffix-to-family`, `sync-gnome-contacts`, `export-json`, `test-connect`, `review-email-label`, `show-email-labels`, the `check-contact-type-company-*` checks, `check-phone-country-label`, `check-contact-no-displayname`).
- `docs/src/same-name-issue.md:5` - refers to a `check-contact-name-duplicate` command (again at `docs/src/same-name-issue.md:53`) that does not exist; the check is `check-contact-displayname-duplicate`.

## Low

- `docs/src/configuration.md:17` - calls the command `check-all` (also `docs/src/commands/merge-by-email.md:57`, `docs/src/commands/merge-by-phone.md:58`); the subcommand is `all-checks` (only the config table is named `[check-all]`). Say `all-checks` when referring to the command.
- `src/commands.rs:4530` - names in `[check-all] skip` are never validated, so a typo silently leaves the check running; reject (or warn about) skip entries that are not known check names.
- `CLAUDE.md:20` - says "Three source files" with all unit tests in `mod tests` in `src/main.rs`, and names a `check-phone-no-label` check (`CLAUDE.md:30`); there is also `src/lib.rs`, tests live in `tests/*.rs`, no `mod tests` exists, and the check is `check-phone-label-missing`. Update it.
- `Cargo.toml:6` - description "Managed your google contacts" is ungrammatical and is what crates.io shows; use e.g. "Audit and fix your Google Contacts".
