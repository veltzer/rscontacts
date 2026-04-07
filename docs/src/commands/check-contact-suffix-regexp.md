# check-contact-suffix-regexp

Check suffixes against the allow regex (default: numeric only).

## Usage

```bash
rscontacts check-contact-suffix-regexp
rscontacts check-contact-suffix-regexp --fix
rscontacts check-contact-suffix-regexp --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix non-matching suffixes |
| `--dry-run` | Show what would change without modifying anything |

## Notes

The allow regex is configured in the `[check-contact-suffix-regexp]` section of `config.toml`. By default, only numeric suffixes are allowed.
