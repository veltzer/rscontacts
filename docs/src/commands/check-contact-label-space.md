# check-contact-label-space

Find contact groups (labels) whose name contains spaces.

## Usage

```bash
rscontacts check-contact-label-space
rscontacts check-contact-label-space --fix
rscontacts check-contact-label-space --fix --dry-run
```

## Fix Behavior

With `--fix`, prompts for a new name for each group. The new name must not be empty or contain spaces.

```
  New name for "My Friends" (or [s]kip):
```
