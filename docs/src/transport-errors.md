# Transport Errors

The Google People API occasionally returns transient HTTP errors that are not caused
by anything wrong with your request. These are server-side issues that typically
resolve themselves after a short wait.

## Error example

Here is an example of a 502 Bad Gateway error returned by the Google People API:

```text
Error: Failure(Response { status: 502, version: HTTP/2.0, headers: {"content-type": "text/html; charset=UTF-8", "referrer-policy": "no-referrer", "content-length": "1613", "date": "Wed, 11 Mar 2026 08:49:16 GMT", "alt-svc": "h3=\":443\"; ma=2592000,h3-29=\":443\"; ma=2592000"}, body: BoxBody })
```

## Retried status codes

rscontacts automatically retries API calls that fail with any of these HTTP status codes:

| Code | Meaning             | Typical cause                              |
|------|---------------------|--------------------------------------------|
| 429  | Too Many Requests   | Rate limit exceeded                        |
| 502  | Bad Gateway         | Google backend overloaded or timed out     |
| 503  | Service Unavailable | Temporary Google service disruption        |
| 504  | Gateway Timeout     | Google backend did not respond in time     |

## Retry behavior

When a transient error is detected, rscontacts retries the request up to 3 times
with exponential backoff delays:

1. First retry after **1 second**
2. Second retry after **2 seconds**
3. Third retry after **4 seconds**

If all 3 retries fail, the error is propagated to the user as usual.

## The `--transport-errors` flag

By default, retries happen silently. To see when retries occur, pass the
`--transport-errors` flag to any command:

```bash
rscontacts --transport-errors list
rscontacts --transport-errors all-checks --fix
```

When a retry is triggered, a message is printed to stderr:

```text
  [transport] HTTP 502 Bad Gateway - retrying in 1s (attempt 1/3)
```

This flag is useful for debugging connectivity issues or monitoring API reliability.
