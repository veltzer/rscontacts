# list

List all contacts with their phone numbers.

## Usage

```bash
rscontacts list
rscontacts list --emails
rscontacts list --labels
rscontacts list --emails --labels
```

## Flags

| Flag | Description |
|------|-------------|
| `--emails` | Also show email addresses |
| `--labels` | Also show phone labels (mobile/home/work) |

## Output Format

Default (name and first phone number):

```
Mark Veltzer | +972-505665636
John Doe
```

With `--emails`:

```
Mark Veltzer | mark@example.com | +972-505665636
```

With `--labels`:

```
Mark Veltzer | +972-505665636 [mobile]
Mark Veltzer | +972-501234567 [work]
```
