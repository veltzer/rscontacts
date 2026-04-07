# edit-contact

Interactively edit a contact.

## Usage

```bash
rscontacts edit-contact "John Doe"
rscontacts edit-contact "John"
```

## Arguments

| Argument | Description |
|----------|-------------|
| `name` | Name (or partial name) of the contact to edit |

## Notes

Performs a case-insensitive substring search on contact names. If multiple contacts match, you will be prompted to select one.
