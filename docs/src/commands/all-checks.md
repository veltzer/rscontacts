# all-checks

Run all checks at once.

## Usage

```bash
rscontacts all-checks
rscontacts all-checks --fix
rscontacts all-checks --fix --dry-run
rscontacts all-checks --stats
rscontacts all-checks --verbose
rscontacts all-checks --country 1
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix all issues found |
| `--dry-run` | Show what would change without modifying anything |
| `--stats` | Only show error counts per check, no details |
| `--verbose`, `-v` | Show verbose output |
| `--country <CODE>` | Country code for phone formatting (default: `972`) |

## Notes

All individual check commands are run in sequence. Each section header includes the corresponding command name so you know which standalone command to use for fixing specific issues.

With `--fix`, all checks support interactive fixing -- the fix/dry-run flags are passed through to every check.
