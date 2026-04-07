# Configuration

rscontacts uses an optional TOML configuration file located at:

```
~/.config/rscontacts/config.toml
```

This is the same directory where OAuth credentials and the token cache are stored.

If the file does not exist, rscontacts runs with default settings (all checks enabled). If the file exists but contains errors, a warning is printed and defaults are used.

You can generate a default config file with `rscontacts init-config`. Use `--force` to overwrite an existing file.

## check-all skip list

The `[check-all]` section controls which checks are included when running `check-all`. By default, all checks run. You can skip specific checks by listing them in the `skip` array:

```toml
[check-all]
skip = [
    "check-contact-given-name-regexp",
    "check-contact-label-nophone",
]
```

Skipped checks will not run and will not appear in the `--stats` output.

Individual check commands (e.g., `rscontacts check-contact-given-name-regexp`) are not affected by the config file and will always run when invoked directly.

## Available check names

The following check names can be used in the `skip` list:

| Check name | Description |
|---|---|
| `check-phone-countrycode` | Phone numbers missing a country code |
| `check-phone-format` | Phone numbers not in +CC-NUMBER format |
| `check-contact-given-name-regexp` | Given names not matching the configured allow regex |
| `check-contact-family-name-regexp` | Family names not matching the configured allow regex |
| `check-contact-suffix-regexp` | Suffixes not matching the allow regex (default: numeric) |
| `check-contact-no-given-name` | Contacts with family name but no given name |
| `check-contact-no-identity` | Contacts with no type tag |
| `check-contact-given-name-known` | Given name not in the configured allowed names list |
| `check-contact-given-name-exists` | Configured given names that have no matching contacts |
| `check-contact-company-known` | Company field not in configured companies list |
| `check-contact-company-exists` | Configured companies that have no matching contacts |
| `check-contact-displayname-duplicate` | Multiple contacts with the same display name |
| `check-contact-no-displayname` | Contacts with empty display name |
| `check-contact-type` | Contacts missing or having both type:Person/type:Company labels |
| `check-contact-type-company-no-company` | Company-tagged contacts without a company field |
| `check-contact-type-company-given-name` | Company-tagged contacts with given name != company field |
| `check-contact-type-company-no-label` | Company-tagged contacts missing company:\<name\> label |
| `check-contact-no-middle-name` | Contacts with a middle name set |
| `check-contact-no-nickname` | Contacts with a nickname set |
| `check-contact-no-label` | Contacts not assigned to any label |
| `check-phone-label-missing` | Phone numbers without a label (mobile/home/work) |
| `check-phone-label-english` | Non-English phone labels |
| `check-phone-country-label` | Missing or wrong country labels for phone numbers |
| `check-contact-email` | Invalid or uppercase email addresses |
| `check-phone-duplicate` | Duplicate phone numbers on a contact |
| `check-contact-email-duplicate` | Duplicate email addresses on a contact |
| `check-contact-label-nophone` | Empty labels (contact groups with no members) |
| `check-contact-label-regexp` | Labels not matching the configured allow regex |

## Config sections

### `[check-all]`

Controls which checks to skip when running `check-all`.

```toml
[check-all]
skip = ["check-contact-no-label", "check-phone-label-missing"]
```

### `[check-contact-given-name-regexp]`

Allowlist regex for given names. Contacts whose given name does **not** match this pattern are flagged. If no `allow` regex is configured, the check is silently skipped in `check-all`.

```toml
[check-contact-given-name-regexp]
allow = '^[A-Z][a-z]*$'
```

### `[check-contact-family-name-regexp]`

Allowlist regex for family names. Same behavior as the given name regex check.

```toml
[check-contact-family-name-regexp]
allow = '^([A-Z][a-z]+(-[A-Z][a-z]+)*|[1-9]\d*)$'
```

### `[check-contact-suffix-regexp]`

Allowlist regex for name suffixes. If not configured, the check uses a default behavior that allows numeric suffixes.

```toml
[check-contact-suffix-regexp]
allow = '^[1-9]\d*$'
```

### `[check-contact-label-regexp]`

Allowlist regex for contact labels (groups). Labels that do **not** match this pattern are flagged. If not configured, the check is silently skipped in `check-all`.

```toml
[check-contact-label-regexp]
allow = '^(type|company|person|service|group|organization):[A-Z][a-zA-Z]*$'
```

### `[check-contact-name-is-company]`

List of known company names. Used by `check-contact-company-known` (flags contacts whose company field is not in this list) and `check-contact-company-exists` (flags configured companies that have no matching contacts). If the list is empty, both checks are skipped in `check-all`.

```toml
[check-contact-name-is-company]
companies = ["Google", "Microsoft", "Amazon"]
```

### `[check-contact-given-name-known]`

List of allowed given names (case-sensitive). Used by `check-contact-given-name-known` (flags contacts whose given name is not in this list) and `check-contact-given-name-exists` (flags configured names that have no matching contacts). If the list is empty, `check-contact-given-name-exists` is skipped in `check-all`.

```toml
[check-contact-given-name-known]
names = ["John", "Jane", "Mark"]
```

## Regex syntax

The `allow` values use [Rust regex syntax](https://docs.rs/regex/latest/regex/#syntax). For example, `'^[A-Z][a-z]+$'` requires values to start with an uppercase letter followed by one or more lowercase letters. Names like "Smith" pass, while "smith", "SMITH", "Smith 2", or "123" would be flagged.

Name regexp checks support `--fix` for interactive fixing (rename/delete/skip, plus swap for given name). The label regexp check supports `--fix` for interactive renaming.

## Example configuration

```toml
# ~/.config/rscontacts/config.toml

[check-all]
skip = [
    "check-contact-given-name-regexp",
    "check-contact-no-label",
    "check-phone-label-missing",
]

[check-contact-given-name-regexp]
allow = '^[A-Z][a-z]*$'

[check-contact-family-name-regexp]
allow = '^([A-Z][a-z]+(-[A-Z][a-z]+)*|[1-9]\d*)$'

[check-contact-label-regexp]
allow = '^(type|company|person|service|group|organization):[A-Z][a-zA-Z]*$'

[check-contact-name-is-company]
companies = ["Google", "Microsoft"]

[check-contact-given-name-known]
names = ["John", "Jane", "Mark"]
```
