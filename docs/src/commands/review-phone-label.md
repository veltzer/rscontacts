# review-phone-label

Review all phones with a specific label (e.g. "Work Fax").

## Usage

```bash
rscontacts review-phone-label "Work Fax"
rscontacts review-phone-label "Work Fax" --fix
rscontacts review-phone-label "Work Fax" --fix --dry-run
```

## Arguments

| Argument | Description |
|----------|-------------|
| `label` | The phone label to review |

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix phone labels |
| `--dry-run` | Show what would change without modifying anything |
