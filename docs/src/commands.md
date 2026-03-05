# Commands

rscontacts provides commands in several categories:

## Listing & Inspection

| Command | Description |
|---------|-------------|
| [list](commands/list.md) | List all contacts |
| [show-contact](commands/show-contact.md) | Show all details about a specific contact |
| [show-phone-labels](commands/show-phone-labels.md) | Show all distinct phone labels in use |
| [show-contact-labels](commands/show-contact-labels.md) | Show all contact groups with member counts |

## Check Commands

All check commands are also run by [check-all](commands/check-all.md).

### Name Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| [check-name-english](commands/check-name-english.md) | Yes | Non-English characters in names |
| [check-name-caps](commands/check-name-caps.md) | Yes | All-caps names |
| [check-name-first-capital-letter](commands/check-name-first-capital-letter.md) | Yes | Names not starting with a capital letter |
| [check-name-order](commands/check-name-order.md) | Yes | Reversed name order ("Family, Given") |

### Phone Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| [check-phone-countrycode](commands/check-phone-countrycode.md) | Yes | Missing country code |
| [check-phone-format](commands/check-phone-format.md) | Yes | Not in `+CC-NUMBER` format |
| [check-phone-no-label](commands/check-phone-no-label.md) | No | Missing phone type label |
| [check-phone-label-english](commands/check-phone-label-english.md) | Yes | Non-English phone labels |

### Email Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| [check-email](commands/check-email.md) | No | Invalid email format |
| [check-email-caps](commands/check-email-caps.md) | Yes | Uppercase letters in emails |
| [check-duplicate-emails](commands/check-duplicate-emails.md) | Yes | Same email attached twice |

### Contact Group (Label) Checks

| Command | `--fix` | Description |
|---------|---------|-------------|
| [check-contact-no-label](commands/check-contact-no-label.md) | No | Contacts not in any group |
| [check-contact-label-space](commands/check-contact-label-space.md) | Yes | Group names containing spaces |
| [check-labels-nophone](commands/check-labels-nophone.md) | Yes | Empty contact groups |
| [check-duplicate-phones](commands/check-duplicate-phones.md) | Yes | Same phone attached twice |

## Other Commands

| Command | Description |
|---------|-------------|
| [auth](commands/auth.md) | Authenticate with Google |
| [version](commands/version.md) | Print version information |
| [complete](commands/complete.md) | Generate shell completions |

## Common Flags

Most check commands support:

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix issues found |
| `--dry-run` | Show what would change without modifying anything |
