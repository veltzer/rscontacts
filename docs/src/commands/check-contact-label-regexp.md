# check-contact-label-regexp

Check contact labels (groups) against the allow regex defined in config.toml.

## Usage

```bash
rscontacts check-contact-label-regexp
rscontacts check-contact-label-regexp --fix
rscontacts check-contact-label-regexp --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix non-matching labels |
| `--dry-run` | Show what would change without modifying anything |
