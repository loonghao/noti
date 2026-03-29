# Webhook Providers

4 providers for generic webhook integrations.

| Provider | Scheme | Description |
|:---------|:-------|:------------|
| Webhook | `webhook://` | Generic HTTP webhook |
| JSON Webhook | `json://` | Generic JSON webhook (POST JSON) |
| Form Webhook | `form://` | Generic form webhook (POST form-encoded) |
| XML Webhook | `xml://` | Generic XML webhook (POST XML) |

## Generic Webhook

```bash
noti send --to "webhook://example.com/api/notify" --message "Hello"
```

Parameters: `url` (required), `method`, `content_type`, `headers`, `body_template` (optional)

## JSON Webhook

Sends a JSON body: `{"message": "...", "title": "...", "type": "info"}`

```bash
noti send --to "json://example.com/api/notify" --message "Hello"
noti send --provider json --param url=https://example.com/api \
  --param header="X-Api-Key=abc" --message "Hello"
```

Parameters: `url` (required), `method` (optional: POST/PUT/PATCH), `header` (optional), `type` (optional)

## Form Webhook

Sends form-encoded body: `message=...&title=...&type=info`

```bash
noti send --to "form://example.com/api/notify" --message "Hello"
```

## XML Webhook

Sends XML body: `<notification><title>...</title><message>...</message></notification>`

```bash
noti send --to "xml://example.com/api/notify" --message "Hello"
```

Parameters: `url` (required), `method`, `header`, `type`, `root` (optional — XML root element name)
