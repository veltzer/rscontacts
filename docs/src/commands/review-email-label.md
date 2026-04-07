# review-email-label

Review all emails with a specific label (e.g. "Work").

## Usage

```bash
rscontacts review-email-label "Work"
rscontacts review-email-label "Work" --fix
rscontacts review-email-label "Work" --fix --dry-run
```

## Arguments

| Argument | Description |
|----------|-------------|
| `label` | The email label to review |

## Flags

| Flag | Description |
|------|-------------|
| `--fix` | Interactively fix email labels |
| `--dry-run` | Show what would change without modifying anything |
