# check-all

Run all checks at once.

## Usage

```bash
rscontacts check-all
rscontacts check-all --fix
rscontacts check-all --fix --dry-run
rscontacts check-all --stats
rscontacts check-all --country 1
```

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix all issues found |
| `--dry-run` | Show what would change without modifying anything |
| `--stats` | Only show error counts per check, no details |
| `--country <CODE>` | Country code for phone formatting (default: `972`) |

## Stats Mode

With `--stats`, only checks with errors are listed:

```
check-phone-format: 5
check-phone-label-english: 3
check-duplicate-emails: 1
---
Total: 9
```

## Checks Run

All individual check commands are run in sequence. Each section header includes the corresponding command name so you know which standalone command to use for fixing specific issues.

With `--fix`, all checks support interactive fixing — the fix/dry-run flags are passed through to every check.
