# check-contact-type

Check that every contact has exactly one of type:Person or type:Company labels.

## Usage

```bash
rscontacts check-contact-type
rscontacts check-contact-type --fix
rscontacts check-contact-type --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix contacts with missing or duplicate type labels |
| `--dry-run` | Show what would change without modifying anything |
