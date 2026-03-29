# SMS & Messaging Providers

17 providers for SMS and messaging services.

| Provider | Scheme | Description |
|:---------|:-------|:------------|
| Twilio | `twilio://` | Twilio SMS via REST API |
| Vonage | `vonage://` | Vonage (Nexmo) SMS API |
| D7 Networks | `d7sms://` | D7 Networks SMS gateway |
| Sinch | `sinch://` | Sinch SMS via REST API |
| Clickatell | `clickatell://` | Clickatell SMS/messaging gateway |
| BulkSMS | `bulksms://` | BulkSMS gateway via REST API v2 |
| Kavenegar | `kavenegar://` | Kavenegar SMS gateway (Iran) |
| MessageBird | `msgbird://` | MessageBird SMS via REST API |
| Plivo | `plivo://` | Plivo SMS via REST API |
| BurstSMS | `burstsms://` | BurstSMS (Transmit SMS) gateway |
| PopcornNotify | `popcorn://` | PopcornNotify SMS messaging |
| ClickSend | `clicksend://` | ClickSend SMS messaging gateway |
| Seven | `seven://` | Seven (sms77) SMS gateway |
| SMSEagle | `smseagle://` | SMSEagle hardware SMS gateway |
| httpSMS | `httpsms://` | httpSMS — send SMS via Android phone |
| MSG91 | `msg91://` | MSG91 SMS gateway (India) |
| Free Mobile | `freemobile://` | Free Mobile SMS (France) |

## Examples

### Twilio

```bash
noti send --to "twilio://<account_sid>:<auth_token>@<from_number>/<to_number>" --message "Hello"
```

### Vonage (Nexmo)

```bash
noti send --to "vonage://<api_key>:<api_secret>@<from_number>/<to_number>" --message "Hello"
```

### D7 Networks

```bash
noti send --to "d7sms://<api_token>@SENDER/+15559876543" --message "Hello"
```
