# check-contact-type-company-given-name

Check that contacts tagged type:Company have given name equal to company field.

## Usage

```bash
rscontacts check-contact-type-company-given-name
rscontacts check-contact-type-company-given-name --fix
rscontacts check-contact-type-company-given-name --auto-fix
rscontacts check-contact-type-company-given-name --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix mismatched given names |
| `--auto-fix` | Automatically set given name to match company field |
| `--dry-run` | Show what would change without modifying anything |
