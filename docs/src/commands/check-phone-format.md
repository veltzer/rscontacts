# check-phone-format

Find phone numbers not in the standard `+CC-NUMBER` format.

The expected format is: `+<country code>-<digits>` with exactly one dash separating the country code from the number, and no other separators. Examples:

- `+972-505665636` (Israel)
- `+1-5551234567` (US)
- `+7-9268335991` (Russia)
- `+44-2079460958` (UK)

## Usage

```bash
rscontacts check-phone-format
rscontacts check-phone-format --fix
rscontacts check-phone-format --fix --country 972
rscontacts check-phone-format --fix --dry-run
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Reformat phone numbers to `+CC-NUMBER` |
| `--dry-run` | Show changes without applying |
| `--country <CODE>` | Default country code for numbers without one (default: `972`) |

## Country Code Detection

When fixing, rscontacts uses a built-in table of ITU country codes to correctly detect the country code length. For example, `+79268335991` is correctly split as `+7-9268335991` (Russia, 1-digit code) rather than `+792-68335991`.

## What Gets Fixed

- Missing dash: `+972505665636` → `+972-505665636`
- Extra dashes: `+972-50-5665636` → `+972-505665636`
- Spaces: `+972 50 566 5636` → `+972-505665636`
- `00` prefix: `00972505665636` → `+972-505665636`
- No country code: `0505665636` → `+972-505665636` (using `--country`)

Non-numeric phone entries (e.g., "VIVINO RLZ") are skipped.
