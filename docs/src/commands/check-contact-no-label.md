# check-contact-no-label

Find contacts that are not assigned to any contact group (label).

## Usage

```bash
rscontacts check-contact-no-label
rscontacts check-contact-no-label --fix
rscontacts check-contact-no-label --fix --dry-run
```

## Fix Behavior

With `--fix`, shows full contact details (name, phones, emails, organization, etc.) and prompts for each unlabeled contact:

```
[l]abel / [d]elete / [s]kip:
```

- **label**: Shows contact details again, then prompts with tab-completion for existing labels. You can also type a new label name — if it doesn't exist, you'll be asked to create it.
- **delete**: Asks for confirmation before deleting the contact.
- **skip**: Moves on to the next contact.

## Notes

In Google Contacts, "labels" are contact groups (e.g., "Friends", "Family", "Work"). This check finds contacts that have no group membership (excluding the default "myContacts" system group).

This is different from [check-phone-no-label](check-phone-no-label.md), which checks phone number type labels.
