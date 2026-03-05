# check-duplicate-phones

Find contacts that have the same phone number attached more than once.

## Usage

```bash
rscontacts check-duplicate-phones
rscontacts check-duplicate-phones --fix
rscontacts check-duplicate-phones --fix --dry-run
```

## Fix Behavior

With `--fix`, prompts for each contact with duplicates:

```
  Remove duplicate "+972-505665636" from John Doe? [y/n]
```

Keeps the first occurrence and removes subsequent duplicates.
