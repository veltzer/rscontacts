# check-contact-given-name-regexp

Check given names against the allow regex defined in config.toml.

## Usage

```bash
rscontacts check-contact-given-name-regexp
rscontacts check-contact-given-name-regexp --fix
rscontacts check-contact-given-name-regexp --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix non-matching given names |
| `--dry-run` | Show what would change without modifying anything |

## Notes

The allow regex is configured in the `[check-contact-given-name-regexp]` section of `config.toml`.
