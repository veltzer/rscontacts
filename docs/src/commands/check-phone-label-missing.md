# check-phone-label-missing

Print contacts with phone numbers missing a label (mobile/home/work/etc).

## Usage

```bash
rscontacts check-phone-label-missing
rscontacts check-phone-label-missing --fix
rscontacts check-phone-label-missing --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively assign labels to unlabeled phone numbers |
| `--dry-run` | Show what would change without modifying anything |

## Notes

Phone "labels" refer to the type field on a phone number (e.g., mobile, home, work). This is distinct from contact labels (contact groups).
