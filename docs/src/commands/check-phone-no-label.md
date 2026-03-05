# check-phone-no-label

Find phone numbers that don't have a type label (mobile, home, work, etc.).

## Usage

```bash
rscontacts check-phone-no-label
rscontacts check-phone-no-label --fix
rscontacts check-phone-no-label --fix --dry-run
```

## Fix Behavior

With `--fix`, prompts with predefined English label choices for each unlabeled phone:

```
Label for John's phone? [m]obile/[h]ome/[w]ork/m[a]in/[o]ther/[s]kip:
```

## Notes

This checks the phone number's `type` and `formatted_type` fields. These are the labels you see in Google Contacts next to each phone number (e.g., "Mobile", "Home", "Work").

This is different from [check-contact-no-label](check-contact-no-label.md), which checks contact group memberships.
