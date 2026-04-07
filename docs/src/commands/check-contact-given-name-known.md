# check-contact-given-name-known

Check that all given names are in the allowed list from config.

## Usage

```bash
rscontacts check-contact-given-name-known
rscontacts check-contact-given-name-known --fix
rscontacts check-contact-given-name-known --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix unknown given names |
| `--dry-run` | Show what would change without modifying anything |

## Notes

The allowed names list is configured in the `[check-contact-given-name-known]` section of `config.toml` via the `names` array.
