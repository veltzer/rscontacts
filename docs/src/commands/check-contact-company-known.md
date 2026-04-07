# check-contact-company-known

Check that every company in the config has at least one contact.

## Usage

```bash
rscontacts check-contact-company-known
rscontacts check-contact-company-known --fix
rscontacts check-contact-company-known --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix companies with no contacts |
| `--dry-run` | Show what would change without modifying anything |
