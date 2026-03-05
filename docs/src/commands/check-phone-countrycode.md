# check-phone-countrycode

Find phone numbers missing a country code.

## Usage

```bash
rscontacts check-phone-countrycode
rscontacts check-phone-countrycode --fix
rscontacts check-phone-countrycode --fix --country 1
rscontacts check-phone-countrycode --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Add country code to matching phone numbers |
| `--dry-run` | Show changes without applying |
| `--country <CODE>` | Country code to prepend (default: `972`) |

## Fix Behavior

Automatically prepends `+<country>` to phone numbers that don't start with `+` or `00`. Leading zeros are stripped (e.g., `0505665636` becomes `+972505665636`).
