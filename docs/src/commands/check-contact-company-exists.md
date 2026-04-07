# check-contact-company-exists

Check that all company fields are in the known companies list from config.

## Usage

```bash
rscontacts check-contact-company-exists
rscontacts check-contact-company-exists --fix
rscontacts check-contact-company-exists --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix contacts with unknown companies |
| `--dry-run` | Show what would change without modifying anything |
