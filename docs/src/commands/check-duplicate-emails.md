# check-duplicate-emails

Find contacts that have the same email address attached more than once.

## Usage

```bash
rscontacts check-duplicate-emails
rscontacts check-duplicate-emails --fix
rscontacts check-duplicate-emails --fix --dry-run
```

## Fix Behavior

With `--fix`, prompts for each contact with duplicates:

```
  Remove duplicate "user@example.com" from John Doe? [y/n]
```

Keeps the first occurrence and removes subsequent duplicates.
