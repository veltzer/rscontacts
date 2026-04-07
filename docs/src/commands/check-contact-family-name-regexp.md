# check-contact-family-name-regexp

Check family names against the allow regex defined in config.toml.

## Usage

```bash
rscontacts check-contact-family-name-regexp
rscontacts check-contact-family-name-regexp --fix
rscontacts check-contact-family-name-regexp --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix non-matching family names |
| `--dry-run` | Show what would change without modifying anything |

## Notes

The allow regex is configured in the `[check-contact-family-name-regexp]` section of `config.toml`. Pure numeric family names are also allowed by default.
