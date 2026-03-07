# Commands

rscontacts provides commands in several categories:

## Listing & Inspection

| Command | Description |
|---------|-------------|
| list | List all contacts |
| show-contact | Show all details about a specific contact |
| edit-contact | Interactively edit a contact |
| show-phone-labels | Show all distinct phone labels in use |
| show-contact-labels | Show all contact groups with member counts |

## Check Commands

All check commands are also run by check-all.

### Name Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| check-contact-given-name-regexp | Yes | Given names not matching allow regex |
| check-contact-family-name-regexp | Yes | Family names not matching allow regex |
| check-contact-suffix-regexp | Yes | Suffixes not matching allow regex |
| check-contact-displayname-duplicate | Yes | Duplicate contact display names |
| check-contact-name-is-company | Yes | Given/family name matches a company name |

### Company Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| check-contact-company-known | Yes | Company field not in configured companies list |

### Phone Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| check-phone-countrycode | Yes | Missing country code |
| check-phone-format | Yes | Not in `+CC-NUMBER` format |
| check-phone-label-missing | Yes | Missing phone type label |
| check-phone-label-english | Yes | Non-English phone labels |
| check-phone-duplicate | Yes | Same phone attached twice |

### Email Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| check-contact-email | Yes | Invalid or uppercase email addresses |
| check-contact-email-duplicate | Yes | Same email attached twice |

### Contact Group (Label) Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| check-contact-no-label | Yes | Contacts not in any group |
| check-contact-label-nophone | Yes | Empty contact groups |
| check-contact-label-regexp | Yes | Labels not matching allow regex |

## Action Commands

| Command | Description |
|---------|-------------|
| compact-suffixes-for-contacts | Compact suffixes for contacts sharing the same base name |
| review-phone-label | Review all phones with a specific label |
| remove-label-from-all-contacts | Remove a label from all contacts |

## Other Commands

| Command | Description |
|---------|-------------|
| auth | Authenticate with Google |
| init-config | Generate a default config file |
| version | Print version information |
| complete | Generate shell completions |

## Common Flags

Most check commands support:

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix issues found |
| `--dry-run` | Show what would change without modifying anything |
