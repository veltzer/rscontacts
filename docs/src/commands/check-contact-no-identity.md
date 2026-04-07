# check-contact-no-identity

Check contacts that have no type tag (type:Person or type:Company).

## Usage

```bash
rscontacts check-contact-no-identity
rscontacts check-contact-no-identity --fix
rscontacts check-contact-no-identity --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively assign a type tag to untagged contacts |
| `--dry-run` | Show what would change without modifying anything |
