# Commands

rscontacts provides commands in several categories. Most check commands support `--fix` and `--dry-run` flags.

## Setup & Diagnostics

- [auth](commands/auth.md) — Authenticate with Google (opens browser for OAuth2 consent)
- [init-config](commands/init-config.md) — Generate a default config file at ~/.config/rscontacts/config.toml
- [test-connect](commands/test-connect.md) — Test connectivity to the Google People API
- [complete](commands/complete.md) — Generate shell completions
- [version](commands/version.md) — Print version information

## Listing & Inspection

- [list](commands/list.md) — List all contacts
- [show-contact](commands/show-contact.md) — Show all details about a specific contact
- [edit-contact](commands/edit-contact.md) — Interactively edit a contact
- [show-contact-labels](commands/show-contact-labels.md) — Show all contact labels (contact groups) in use
- [show-email-labels](commands/show-email-labels.md) — Show all distinct email labels in use
- [show-phone-labels](commands/show-phone-labels.md) — Show all distinct phone labels in use
- [export-json](commands/export-json.md) — Export all contacts as JSON

## Run All Checks

- [all-checks](commands/all-checks.md) — Run all checks

## Name Checks

- [check-contact-given-name-regexp](commands/check-contact-given-name-regexp.md) — Check given names against allow regex defined in config.toml
- [check-contact-family-name-regexp](commands/check-contact-family-name-regexp.md) — Check family names against allow regex defined in config.toml
- [check-contact-suffix-regexp](commands/check-contact-suffix-regexp.md) — Check suffixes against allow regex (default: numeric)
- [check-contact-displayname-duplicate](commands/check-contact-displayname-duplicate.md) — Print contacts that share the same display name
- [check-contact-no-displayname](commands/check-contact-no-displayname.md) — Print contacts with an empty display name
- [check-contact-no-given-name](commands/check-contact-no-given-name.md) — Check contacts that have no given name but have a family name
- [check-contact-no-middle-name](commands/check-contact-no-middle-name.md) — Check that no contact has a middle name set
- [check-contact-no-nickname](commands/check-contact-no-nickname.md) — Check that no contact has a nickname set
- [check-contact-given-name-known](commands/check-contact-given-name-known.md) — Check that all given names are in the allowed list from config
- [check-contact-given-name-exists](commands/check-contact-given-name-exists.md) — Check that every given name in the config has at least one contact

## Company Checks

- [check-contact-company-exists](commands/check-contact-company-exists.md) — Check that all company fields are in the known companies list from config
- [check-contact-company-known](commands/check-contact-company-known.md) — Check that every company in the config has at least one contact
- [check-contact-type](commands/check-contact-type.md) — Check that every contact has exactly one of type:Person or type:Company labels
- [check-contact-no-identity](commands/check-contact-no-identity.md) — Check contacts that have no type tag (type:Person or type:Company)
- [check-contact-type-company-given-name](commands/check-contact-type-company-given-name.md) — Check that contacts tagged type:Company have given name equal to company field
- [check-contact-type-company-no-company](commands/check-contact-type-company-no-company.md) — Check that contacts tagged type:Company have their company field set
- [check-contact-type-company-no-label](commands/check-contact-type-company-no-label.md) — Check that contacts tagged type:Company have a matching company:\<name\> label

## Phone Checks

- [check-phone-countrycode](commands/check-phone-countrycode.md) — Print contacts with phone numbers missing a country code
- [check-phone-format](commands/check-phone-format.md) — Print phone numbers not in +CC-NUMBER format
- [check-phone-duplicate](commands/check-phone-duplicate.md) — Print contacts that have the same phone number attached twice
- [check-phone-label-missing](commands/check-phone-label-missing.md) — Print contacts with phone numbers missing a label (mobile/home/work/etc)
- [check-phone-label-english](commands/check-phone-label-english.md) — Print contacts with non-English phone labels
- [check-phone-country-label](commands/check-phone-country-label.md) — Check that contacts have correct country:\<Name\> labels matching their phone country codes

## Email Checks

- [check-contact-email](commands/check-contact-email.md) — Print contacts with invalid or uppercase email addresses
- [check-contact-email-duplicate](commands/check-contact-email-duplicate.md) — Print contacts that have the same email address attached twice

## Contact Group (Label) Checks

- [check-contact-no-label](commands/check-contact-no-label.md) — Print contacts not assigned to any label (contact group)
- [check-contact-label-nophone](commands/check-contact-label-nophone.md) — Print labels (contact groups) that have no contacts
- [check-contact-label-regexp](commands/check-contact-label-regexp.md) — Check contact labels (groups) against allow regex defined in config.toml

## Merge Commands

- [merge-by-phone](commands/merge-by-phone.md) — Find and merge contacts that share the same phone number
- [merge-by-email](commands/merge-by-email.md) — Find and merge contacts that share the same email address

## Action Commands

- [move-family-to-suffix](commands/move-family-to-suffix.md) — Move numeric family names to suffix for contacts that have no suffix
- [move-suffix-to-family](commands/move-suffix-to-family.md) — Move numeric suffixes to family name for contacts that have no family name
- [compact-suffixes-for-contacts](commands/compact-suffixes-for-contacts.md) — Compact suffixes for contacts sharing the same base name (given + family)
- [remove-label-from-all-contacts](commands/remove-label-from-all-contacts.md) — Remove a contact label (group) from all contacts that have it
- [review-phone-label](commands/review-phone-label.md) — Review all phones with a specific label (e.g. "Work Fax")
- [review-email-label](commands/review-email-label.md) — Review all emails with a specific label (e.g. "Work")
- [sync-gnome-contacts](commands/sync-gnome-contacts.md) — Sync Google Contacts to GNOME Contacts (Evolution Data Server)
