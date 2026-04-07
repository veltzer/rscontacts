# check-contact-type-company-no-company

Check that contacts tagged type:Company have their company field set.

## Usage

```bash
rscontacts check-contact-type-company-no-company
rscontacts check-contact-type-company-no-company --fix
rscontacts check-contact-type-company-no-company --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix company contacts missing the company field |
| `--dry-run` | Show what would change without modifying anything |
