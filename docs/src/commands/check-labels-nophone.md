# check-labels-nophone

Find contact groups (labels) that have no contacts assigned to them.

## Usage

```bash
rscontacts check-labels-nophone
rscontacts check-labels-nophone --fix
rscontacts check-labels-nophone --fix --dry-run
```

## Fix Behavior

With `--fix`, prompts to delete each empty group:

```
  Delete label "OldGroup"? [y/n]
```
