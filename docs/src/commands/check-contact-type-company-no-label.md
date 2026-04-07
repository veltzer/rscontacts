# check-contact-type-company-no-label

Check that contacts tagged type:Company have a matching company:<name> label.

## Usage

```bash
rscontacts check-contact-type-company-no-label
rscontacts check-contact-type-company-no-label --fix
rscontacts check-contact-type-company-no-label --auto-fix
rscontacts check-contact-type-company-no-label --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix missing company labels |
| `--auto-fix` | Automatically create and assign the company label |
| `--dry-run` | Show what would change without modifying anything |
