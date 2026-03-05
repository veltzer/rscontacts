# check-name-order

Find contacts with reversed name order (e.g., "Veltzer, Mark" instead of "Mark Veltzer").

This happens when Google formats `display_name` with the family name first, typically for contacts created with a non-Latin locale.

## Usage

```bash
rscontacts check-name-order
rscontacts check-name-order --fix
rscontacts check-name-order --fix --dry-run
```

## Output

```
Veltzer, Mark -> Mark Veltzer
```

## Fix Behavior

With `--fix`, prompts for each contact: **[r]ename / [d]elete / [s]kip**. When renaming, the suggested corrected name is offered as a default (press Enter to accept).
