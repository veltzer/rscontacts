# check-email

Find contacts with invalid-looking email addresses.

## Usage

```bash
rscontacts check-email
```

## Validation Rules

An email is considered invalid if:

- It has no `@` sign
- The local part (before `@`) is empty
- The domain part (after `@`) is empty or has no `.`
- The TLD is less than 2 characters
