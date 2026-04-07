# check-contact-given-name-exists

Check that every given name in the config has at least one contact.

## Usage

```bash
rscontacts check-contact-given-name-exists
rscontacts check-contact-given-name-exists --fix
rscontacts check-contact-given-name-exists --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix given names with no contacts |
| `--dry-run` | Show what would change without modifying anything |
