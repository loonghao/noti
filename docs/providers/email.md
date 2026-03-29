# Email Providers

8 providers for transactional email services.

| Provider | Scheme | Description |
|:---------|:-------|:------------|
| Email | `smtp://` | Email via SMTP |
| Mailgun | `mailgun://` | Mailgun transactional email API |
| SendGrid | `sendgrid://` | SendGrid transactional email API v3 |
| SparkPost | `sparkpost://` | SparkPost transactional email API |
| AWS SES | `ses://` | AWS SES transactional email |
| Resend | `resend://` | Resend modern email API |
| Brevo | `brevo://` | Brevo (Sendinblue) transactional email |
| SMTP2Go | `smtp2go://` | SMTP2Go transactional email |

## Examples

### Email (SMTP)

```bash
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=addr@example.com" \
  --message "Hello" --title "Subject"
```

Parameters: `host`, `username`, `password`, `to` (required), `port`, `from` (optional)

### Mailgun

```bash
noti send --to "mailgun://<api_key>@<domain>/<to_email>" --message "Hello" --title "Subject"
```

### SendGrid

```bash
noti send --to "sendgrid://<api_key>@sender@example.com/recipient@example.com" \
  --message "Hello" --title "Subject"
```

### Resend

```bash
noti send --to "resend://re_123abc@noti@yourdomain.com/user@example.com" \
  --message "Hello" --title "Subject"
```

### AWS SES

```bash
noti send --to "ses://<access_key>:<secret_key>@us-east-1/sender@example.com/recipient@example.com" \
  --message "Hello" --title "Subject"
```
