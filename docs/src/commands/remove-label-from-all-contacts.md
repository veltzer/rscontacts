# remove-label-from-all-contacts

Remove a contact label (group) from all contacts that have it.

## Usage

```bash
rscontacts remove-label-from-all-contacts "Old Label"
rscontacts remove-label-from-all-contacts "Old Label" --dry-run
```

## Arguments

| Argument | Description |
|----------|-------------|
| `label` | The label (contact group) name to remove |

## Flags

| Flag | Description |
|------|-------------|
| `--dry-run` | Show what would change without modifying anything |
