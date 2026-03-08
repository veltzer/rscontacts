# rscontacts

A command-line tool for auditing and fixing Google Contacts, written in Rust.

rscontacts connects to the Google People API and runs a suite of checks on your contacts to find issues like duplicate phone numbers, missing labels, invalid email addresses, inconsistent name formatting, and more. Most checks support interactive `--fix` mode to resolve issues on the spot.

## Installation

```bash
cargo install --path .
```

Requires Rust edition 2024.

## Setup

1. **Create a Google Cloud project** with the People API enabled.
2. **Download OAuth2 credentials** (Desktop app type) and save as:
   ```
   ~/.config/rscontacts/credentials.json
   ```
3. **Authenticate:**
   ```bash
   rscontacts auth
   ```
   This opens a browser for OAuth2 consent. Use `--no-browser` to print the URL instead.

4. **Initialize config** (optional):
   ```bash
   rscontacts init-config
   ```
   Creates `~/.config/rscontacts/config.toml` with default settings.

## Usage

### List contacts

```bash
rscontacts list                  # Basic listing
rscontacts list --labels         # Include contact group memberships
rscontacts list --emails         # Include email addresses
rscontacts list --starred        # Only starred contacts
```

### Run all checks

```bash
rscontacts all-checks            # Report all issues
rscontacts all-checks --fix      # Interactively fix all issues
rscontacts all-checks --stats    # Show only issue counts per check
rscontacts all-checks --verbose  # Print each check name as it runs
```

### Individual checks

Each check can be run standalone with optional `--fix` and `--dry-run` flags:

| Command | Description |
|---------|-------------|
| `check-contact-given-name-regexp` | Given names matching a configurable regex |
| `check-contact-family-name-regexp` | Family names matching a configurable regex |
| `check-contact-suffix-regexp` | Name suffixes matching a configurable regex |
| `check-contact-no-given-name` | Contacts with a family name but no given name |
| `check-contact-no-identity` | Contacts missing any `type:*` tag |
| `check-contact-type` | Contacts with incorrect type:Person/type:Company tagging |
| `check-contact-given-name-known` | Given names not in the configured allow list |
| `check-contact-company-known` | Company names not in the configured allow list |
| `check-contact-given-name-exists` | Config names with no matching contact |
| `check-contact-company-exists` | Config companies with no matching contact |
| `check-contact-displayname-duplicate` | Contacts sharing the same display name |
| `check-contact-no-label` | Contacts not in any contact group |
| `check-contact-no-middle-name` | Contacts with a middle name set |
| `check-contact-no-nickname` | Contacts with a nickname set |
| `check-phone-countrycode` | Phone numbers missing a country code |
| `check-phone-format` | Phone numbers not in `+CC-NUMBER` format |
| `check-phone-label-missing` | Phone numbers without a label (mobile/home/work) |
| `check-phone-label-english` | Phone numbers with non-English labels |
| `check-phone-duplicate` | Duplicate phone numbers on the same contact |
| `check-contact-email` | Invalid or uppercase email addresses |
| `check-contact-email-duplicate` | Duplicate email addresses on the same contact |
| `check-contact-label-nophone` | Empty contact groups (labels with no members) |
| `check-contact-label-regexp` | Contact group names not matching the configured regex |

### Processor commands

| Command | Description |
|---------|-------------|
| `auto-contact-type` | Auto-assign type:Person or type:Company based on name fields |
| `company-labels` | Prefix labels of company contacts with `company:` |
| `fix-labels` | Interactively fix labels that don't match the configured regex |
| `compact-suffixes-for-contacts` | Renumber suffixes for contacts sharing a base name |
| `move-given-name-to-company` | Move a given name to the company field |
| `remove-label-from-all-contacts` | Remove a label from all contacts that have it |
| `review-phone-label` | Review all phones with a specific label |

### Utility commands

| Command | Description |
|---------|-------------|
| `list` | List all contacts |
| `show-contact <name>` | Show full details for a contact |
| `edit-contact <name>` | Interactively edit a contact |
| `show-phone-labels` | Show all distinct phone labels in use |
| `show-contact-labels` | Show all contact groups |
| `auth` | Authenticate with Google |
| `init-config` | Generate default config file |
| `version` | Print version and build info |
| `complete <shell>` | Generate shell completions (bash/zsh/fish/etc.) |

## Configuration

Config file: `~/.config/rscontacts/config.toml`

```toml
[check-all]
# Skip specific checks in all-checks
skip = [
    # "check-contact-given-name-regexp",
    # "check-phone-format",
]

[check-contact-given-name-regexp]
allow = '^[A-Z][a-z]*$'

[check-contact-family-name-regexp]
allow = '^[A-Z][a-z]+(-[A-Z][a-z]+)*$'

[check-contact-label-regexp]
allow = '^(type|company|person|service|group|organization):[A-Z][a-zA-Z]*$'

[check-contact-name-is-company]
companies = ["CompanyA", "CompanyB"]

[check-contact-given-name-known]
names = ["Alice", "Bob", "Charlie"]
```

## Building

```bash
cargo build                    # Debug build
cargo build --release          # Release build (stripped, LTO, single codegen unit)
cargo clippy                   # Lint
cargo nextest run              # Run tests
```

## License

See [LICENSE](LICENSE) for details.
