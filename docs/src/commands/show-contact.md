# show-contact

Show all available details about a specific contact.

## Usage

```bash
rscontacts show-contact "John"
rscontacts show-contact "Doe"
```

## How It Works

Performs a case-insensitive substring search on contact names. If multiple contacts match, all are displayed separated by a divider.

## Fields Displayed

- Name (given, family, middle, prefix, suffix)
- Nicknames
- Email addresses (with type)
- Phone numbers (with type)
- Addresses
- Organizations (title, company, department)
- Birthdays
- Relations
- Events
- Biographies
- URLs
- IM clients
- SIP addresses
- Occupations, interests, skills
- Locations
- External IDs
- Custom fields and client data
- Contact group memberships (labels)
- Resource name (internal Google ID)
