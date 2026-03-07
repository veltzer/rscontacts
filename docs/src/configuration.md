# Configuration

rscontacts uses an optional TOML configuration file located at:

```
~/.config/rscontacts/config.toml
```

This is the same directory where OAuth credentials and the token cache are stored.

If the file does not exist, rscontacts runs with default settings (all checks enabled). If the file exists but contains errors, a warning is printed and defaults are used.

## check-all skip list

The `[check-all]` section controls which checks are included when running `check-all`. By default, all checks run. You can skip specific checks by listing them in the `skip` array:

```toml
[check-all]
skip = [
    "check-contact-name-english",
    "check-contact-label-nophone",
]
```

Skipped checks will not run and will not appear in the `--stats` output.

Individual check commands (e.g., `rscontacts check-contact-name-english`) are not affected by the config file and will always run when invoked directly.

## Available check names

The following check names can be used in the `skip` list:

| Check name | Description |
|---|---|
| `check-contact-name-english` | Non-English contact names |
| `check-contact-name-caps` | All-caps contact names |
| `check-contact-name-first-capital-letter` | Names not starting with a capital letter |
| `check-contact-firstname-regexp` | First names not matching the configured allow regex |
| `check-contact-lastname-regexp` | Last names not matching the configured allow regex |
| `check-contact-suffix-regexp` | Suffixes not matching the allow regex (default: numeric) |
| `check-contact-name-order` | Reversed name order (e.g., "Family, Given") |
| `check-contact-displayname-duplicate` | Multiple contacts with the same display name |
| `check-contact-name-numeric-surname` | Numeric surnames (should be suffixes) |
| `check-contact-samename-suffix` | Same-name contacts with missing or non-sequential suffixes |
| `check-contact-no-label` | Contacts not assigned to any label |
| `check-contact-email` | Invalid email addresses |
| `check-contact-email-caps` | Email addresses with uppercase letters |
| `check-contact-email-duplicate` | Duplicate email addresses on a contact |
| `check-contact-label-nophone` | Empty labels (contact groups with no members) |
| `check-contact-label-space` | Labels with spaces in their name |
| `check-contact-label-camelcase` | Labels not starting with a capital letter |
| `check-phone-countrycode` | Phone numbers missing a country code |
| `check-phone-format` | Phone numbers not in +CC-NUMBER format |
| `check-phone-no-label` | Phone numbers without a label (mobile/home/work) |
| `check-phone-label-english` | Non-English phone labels |
| `check-phone-duplicate` | Duplicate phone numbers on a contact |

## Name allow regexes

The `check-contact-firstname-regexp` and `check-contact-lastname-regexp` checks flag contacts whose first name (given name) or last name (family name) does **not** match the configured regex pattern. This is an allowlist approach — define what a valid name looks like, and anything that doesn't match gets flagged.

```toml
[check-contact-firstname-regexp]
allow = '^[A-Z][a-z]+$'

[check-contact-lastname-regexp]
allow = '^[A-Z][a-z]+$'
```

The `allow` value is a [Rust regex](https://docs.rs/regex/latest/regex/#syntax). The example above requires names to start with an uppercase letter followed by one or more lowercase letters. Names like "Smith" pass, while "smith", "SMITH", "Smith 2", or "123" would be flagged.

If no `allow` regex is configured, the check is silently skipped in `check-all`. When run directly, it prints a message about the missing config.

Both checks support `--fix` for interactive fixing (rename/delete/skip, plus swap for firstname).

## Example configuration

```toml
# ~/.config/rscontacts/config.toml

[check-all]
skip = [
    "check-contact-name-english",
    "check-contact-no-label",
    "check-phone-no-label",
]

[check-contact-firstname-regexp]
allow = '^[A-Z][a-z]+$'

[check-contact-lastname-regexp]
allow = '^[A-Z][a-z]+$'
```
