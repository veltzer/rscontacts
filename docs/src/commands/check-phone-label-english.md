# check-phone-label-english

Find phone numbers whose type label contains non-English (non-ASCII) characters, such as "Мобильный" (Russian) or "נייד" (Hebrew) instead of "mobile".

## Usage

```bash
rscontacts check-phone-label-english
rscontacts check-phone-label-english --fix
rscontacts check-phone-label-english --fix --dry-run
```

## Output

```
Alex | +972542518077 [Мобильный]
Oren | +972528478018 [נייד]
```

## Fix Behavior

With `--fix`, prompts with predefined English label choices:

```
Label for Alex's phone? [m]obile/[h]ome/[w]ork/m[a]in/[o]ther/[s]kip:
```
