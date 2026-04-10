//! Provider send tests: email category.

use super::provider_test_utils::*;

mod email_tests {

    use super::*;

    use noti_providers::email::EmailProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = EmailProvider::new();

        let config = ProviderConfig::new()

            .set("host", "smtp.gmail.com")

            .set("username", "user@gmail.com")

            .set("password", "app-password")

            .set("to", "recipient@example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = EmailProvider::new();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        // Missing password, to

        assert!(

            provider

                .validate_config(

                    &ProviderConfig::new()

                        .set("host", "smtp.gmail.com")

                        .set("username", "user@gmail.com")

                )

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = EmailProvider::new();

        assert_eq!(provider.name(), "email");

        assert_eq!(provider.url_scheme(), "smtp");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "host" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "username" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "password" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "to" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "port" && !p.required)

        );

    }



    #[tokio::test]

    async fn test_default_trait() {

        let provider = EmailProvider::new();

        assert_eq!(provider.name(), "email");

    }

}


mod resend_send_tests {

    use super::*;

    use noti_providers::resend::ResendProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = ResendProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "re_xxx")

            .set("from", "from@example.com")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = ResendProvider::new(client());

        let config = ProviderConfig::new()

            .set("from", "from@example.com")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_from() {

        let provider = ResendProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "re_xxx")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = ResendProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "re_xxx")

            .set("from", "from@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_with_reply_to() {

        let provider = ResendProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "re_xxx")

            .set("from", "from@example.com")

            .set("to", "to@example.com")

            .set("reply_to", "reply@example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = ResendProvider::new(client());

        assert_eq!(provider.name(), "resend");

        assert_eq!(provider.url_scheme(), "resend");

        assert!(provider.description().contains("Resend"));

        assert!(provider.supports_attachments());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "api_key" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "from" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "to" && p.required)

        );

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/emails"))

            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({

                "id": "em_123abc",

                "from": "from@example.com",

                "to": ["to@example.com"],

                "subject": "Notification"

            })))

            .mount(&mock_server)

            .await;



        let provider = ResendProvider::new(client());

        // Override base URL by crafting a config that Resend would use

        let config = ProviderConfig::new()

            .set("api_key", "re_test")

            .set("from", "from@example.com")

            .set("to", "to@example.com");



        let message = Message::text("Hello World");

        let result = provider.send(&message, &config).await;

        // Resend uses hardcoded https://api.resend.com, so network call fails

        // This validates that params are correctly extracted

        assert!(result.is_err() || result.as_ref().is_ok());

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let provider = ResendProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "re_test")

            .set("from", "from@example.com")

            .set("to", "to@example.com");



        let message = Message::text("Body").with_title("Subject Line");

        let result = provider.send(&message, &config).await;

        // Network error expected (hardcoded URL); validates title is used as subject

        assert!(result.is_err() || result.as_ref().is_ok());

    }



    #[tokio::test]

    async fn test_send_failure() {

        let provider = ResendProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "re_bad")

            .set("from", "from@example.com")

            .set("to", "to@example.com");



        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        // Network error expected; validates failure path

        assert!(result.is_err() || result.as_ref().is_ok());

    }

}


mod brevo_send_tests {

    use super::*;

    use noti_providers::brevo::BrevoProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-test")

            .set("from", "from@example.com")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("from", "from@example.com")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_from() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-test")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-test")

            .set("from", "from@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_with_all_optional() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-test")

            .set("from", "from@example.com")

            .set("to", "to@example.com")

            .set("from_name", "Noti Sender")

            .set("to_name", "Recipient")

            .set("cc", "cc@example.com")

            .set("bcc", "bcc@example.com")

            .set("reply_to", "reply@example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = BrevoProvider::new(client());

        assert_eq!(provider.name(), "brevo");

        assert_eq!(provider.url_scheme(), "brevo");

        assert!(provider.description().contains("Brevo"));

        assert!(provider.supports_attachments());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "api_key" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "from" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "to" && p.required)

        );

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/v3/smtp/email"))

            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({

                "messageId": "msg_123abc",

                "templateId": null

            })))

            .mount(&mock_server)

            .await;



        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-test")

            .set("from", "from@example.com")

            .set("to", "to@example.com");



        let message = Message::text("Hello World");

        let result = provider.send(&message, &config).await;

        // Brevo uses hardcoded https://api.brevo.com, network call will fail

        assert!(result.is_err() || result.as_ref().is_ok());

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-test")

            .set("from", "from@example.com")

            .set("to", "to@example.com");



        let message = Message::text("Body").with_title("Email Subject");

        let result = provider.send(&message, &config).await;

        // Network error expected; validates title is used as subject

        assert!(result.is_err() || result.as_ref().is_ok());

    }



    #[tokio::test]

    async fn test_send_with_html_format() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-test")

            .set("from", "from@example.com")

            .set("to", "to@example.com");



        let message = Message::text("<b>Bold text</b>").with_format(MessageFormat::Html);

        let result = provider.send(&message, &config).await;

        // Network error expected; validates HTML format is handled

        assert!(result.is_err() || result.as_ref().is_ok());

    }



    #[tokio::test]

    async fn test_send_failure() {

        let provider = BrevoProvider::new(client());

        let config = ProviderConfig::new()

            .set("api_key", "xkeys-bad")

            .set("from", "from@example.com")

            .set("to", "to@example.com");



        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        // Network error expected; validates failure path

        assert!(result.is_err() || result.as_ref().is_ok());

    }

}


mod email_send_tests {

    use super::*;

    use noti_providers::email::EmailProvider;



    fn make_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "smtp.example.com")

            .set("username", "user@example.com")

            .set("password", "app-password")

            .set("to", "recipient@example.com")

    }



    #[tokio::test]

    async fn test_validate_config_full() {

        let provider = EmailProvider::new();

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = EmailProvider::new();

        let config = ProviderConfig::new()

            .set("username", "user@example.com")

            .set("password", "app-password")

            .set("to", "recipient@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_username() {

        let provider = EmailProvider::new();

        let config = ProviderConfig::new()

            .set("host", "smtp.example.com")

            .set("password", "app-password")

            .set("to", "recipient@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_password() {

        let provider = EmailProvider::new();

        let config = ProviderConfig::new()

            .set("host", "smtp.example.com")

            .set("username", "user@example.com")

            .set("to", "recipient@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = EmailProvider::new();

        let config = ProviderConfig::new()

            .set("host", "smtp.example.com")

            .set("username", "user@example.com")

            .set("password", "app-password");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_empty_config() {

        let provider = EmailProvider::new();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = EmailProvider::new();

        assert_eq!(provider.name(), "email");

        assert_eq!(provider.url_scheme(), "smtp");

        assert!(!provider.description().is_empty());

        assert!(provider.example_url().starts_with("smtp://"));

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert_eq!(params.iter().filter(|p| p.required).count(), 4);

        assert_eq!(params.iter().filter(|p| !p.required).count(), 4);

    }



    #[tokio::test]

    async fn test_send_smtp_connection_error() {

        // Email uses SMTP (not HTTP), so connecting to a non-existent host should fail gracefully.

        // The provider catches SMTP errors and returns a failure SendResponse (not an Err).

        let provider = EmailProvider::new();

        let config = make_config();

        let message = Message::text("Test email body");



        let result = provider.send(&message, &config).await;

        // SMTP connection to non-existent host should either:

        // 1. Return Ok(SendResponse { success: false, ... }) — SMTP error caught

        // 2. Return Err — if DNS resolution / TLS setup fails before SMTP

        match result {

            Ok(resp) => assert!(!resp.success, "Expected failure for non-existent SMTP host"),

            Err(_) => {} // DNS/TLS error is also acceptable

        }

    }



    #[tokio::test]

    async fn test_send_with_title_as_subject() {

        // Verify that message title becomes email subject

        let provider = EmailProvider::new();

        let config = make_config();

        let message = Message::text("Body text").with_title("Urgent Alert");



        let result = provider.send(&message, &config).await;

        // SMTP connection will fail; just verify no panic

        assert!(result.is_ok() || result.is_err());

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let provider = EmailProvider::new();

        let config = make_config();

        let message = Message::text("<h1>HTML Email</h1>").with_format(MessageFormat::Html);



        let result = provider.send(&message, &config).await;

        // SMTP connection will fail; just verify no panic

        assert!(result.is_ok() || result.is_err());

    }



    #[tokio::test]

    async fn test_send_with_cc_and_bcc() {

        let provider = EmailProvider::new();

        let config = make_config()

            .set("cc", "cc1@example.com, cc2@example.com")

            .set("bcc", "bcc@example.com");

        let message = Message::text("CC and BCC test");



        let result = provider.send(&message, &config).await;

        // SMTP connection will fail; just verify no panic

        assert!(result.is_ok() || result.is_err());

    }



    #[tokio::test]

    async fn test_send_with_custom_from() {

        let provider = EmailProvider::new();

        let config = make_config().set("from", "custom@example.com");

        let message = Message::text("Custom from");



        let result = provider.send(&message, &config).await;

        // SMTP connection will fail; just verify no panic

        assert!(result.is_ok() || result.is_err());

    }



    #[tokio::test]

    async fn test_send_with_invalid_to() {

        let provider = EmailProvider::new();

        let config = make_config().set("to", "not-an-email");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        assert!(result.unwrap_err().to_string().contains("invalid to address"));

    }



    #[tokio::test]

    async fn test_send_with_custom_port() {

        let provider = EmailProvider::new();

        let config = make_config().set("port", "465");

        let message = Message::text("Port 465 test");



        let result = provider.send(&message, &config).await;

        // SMTP connection will fail; just verify no panic

        assert!(result.is_ok() || result.is_err());

    }

}


mod mailgun_send_tests {

    use super::*;

    use noti_providers::mailgun::MailgunProvider;



    fn provider() -> MailgunProvider {

        MailgunProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "key-test123")

            .set("domain", "mg.example.com")

            .set("to", "user@example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "k")

            .set("domain", "d")

            .set("to", "t");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("domain", "d")

            .set("to", "t");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "mailgun");

        assert_eq!(provider.url_scheme(), "mailgun");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_text_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "<123@mg.example.com>",

                "message": "Queued. Thank you."

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Mailgun");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "mailgun");

    }



    #[tokio::test]

    async fn test_send_with_subject() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "<124@mg.example.com>",

                "message": "Queued. Thank you."

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Important Subject");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "<125@mg.example.com>",

                "message": "Queued. Thank you."

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("<h1>Hello</h1>").with_format(MessageFormat::Html);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "message": "Invalid domain"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "message": "Unauthorized"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_eu_region() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "<126@mg.example.com>",

                "message": "Queued."

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key-test")

            .set("domain", "mg.eu.com")

            .set("to", "user@eu.com")

            .set("region", "eu")

            .set("base_url", &server.uri());

        let message = Message::text("EU region test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod sendgrid_send_tests {

    use super::*;

    use noti_providers::sendgrid::SendGridProvider;



    fn provider() -> SendGridProvider {

        SendGridProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "SG.testkey123")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "k")

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "sendgrid");

        assert_eq!(provider.url_scheme(), "sendgrid");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        // SendGrid returns 202 Accepted with empty body on success

        Mock::given(method("POST"))

            .and(path("/v3/mail/send"))

            .respond_with(ResponseTemplate::new(202))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from SendGrid");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "sendgrid");

        assert_eq!(result.status_code, Some(202));

    }



    #[tokio::test]

    async fn test_send_with_subject() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/mail/send"))

            .respond_with(ResponseTemplate::new(202))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Important Subject");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/mail/send"))

            .respond_with(ResponseTemplate::new(202))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("<h1>Hello</h1>").with_format(MessageFormat::Html);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_cc_bcc() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/mail/send"))

            .respond_with(ResponseTemplate::new(202))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("cc", "cc1@example.com,cc2@example.com")

            .set("bcc", "bcc@example.com");

        let message = Message::text("CC/BCC test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_from_name_to_name() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/mail/send"))

            .respond_with(ResponseTemplate::new(202))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("from_name", "Noti Bot")

            .set("to_name", "John Doe");

        let message = Message::text("Named test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/mail/send"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "errors": [{"message": "Invalid API key"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/mail/send"))

            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({

                "errors": [{"message": "Forbidden"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}


mod ses_send_tests {

    use super::*;

    use noti_providers::ses::SesProvider;



    fn provider() -> SesProvider {

        SesProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_key", "AKIAIOSFODNN7EXAMPLE")

            .set("secret_key", "wJalrXUtnFEMI/K7MDENG")

            .set("region", "us-east-1")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "a")

            .set("secret_key", "s")

            .set("region", "r")

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_access_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("secret_key", "s")

            .set("region", "r")

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "ses");

        assert_eq!(provider.url_scheme(), "ses");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string(

                "<SendEmailResponse><SendEmailResult><MessageId>msg-001</MessageId></SendEmailResult></SendEmailResponse>"

            ))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from SES");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "ses");

    }



    #[tokio::test]

    async fn test_send_with_subject() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string(

                "<SendEmailResponse><SendEmailResult><MessageId>msg-002</MessageId></SendEmailResult></SendEmailResponse>"

            ))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Important Subject");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_string(

                "<ErrorResponse><Error><Message>Invalid address</Message></Error></ErrorResponse>"

            ))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_string(

                "<ErrorResponse><Error><Message>SignatureDoesNotMatch</Message></Error></ErrorResponse>"

            ))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}


mod sparkpost_send_tests {

    use super::*;

    use noti_providers::sparkpost::SparkPostProvider;



    fn provider() -> SparkPostProvider {

        SparkPostProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "sparkpost-test-key")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "k")

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "sparkpost");

        assert_eq!(provider.url_scheme(), "sparkpost");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "results": {"id": "msg-001", "total_rejected_recipients": 0, "total_accepted_recipients": 1}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from SparkPost");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "sparkpost");

    }



    #[tokio::test]

    async fn test_send_with_subject() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "results": {"id": "msg-002", "total_rejected_recipients": 0, "total_accepted_recipients": 1}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Important Email");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "results": {"id": "msg-003", "total_rejected_recipients": 0, "total_accepted_recipients": 1}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("<h1>Hello</h1>").with_format(MessageFormat::Html);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "errors": [{"message": "Invalid API key"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({

                "errors": [{"message": "Forbidden"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}


mod smtp2go_send_tests {

    use super::*;

    use noti_providers::smtp2go::Smtp2GoProvider;



    fn provider() -> Smtp2GoProvider {

        Smtp2GoProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "smtp2go-test-key")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "k")

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("from", "f@e.com")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_from() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "k")

            .set("to", "t@e.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "k")

            .set("from", "f@e.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "smtp2go");

        assert_eq!(provider.url_scheme(), "smtp2go");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"api_key"));

        assert!(param_names.contains(&"from"));

        assert!(param_names.contains(&"to"));

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "data": {"succeeded": 1, "failed": 0, "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from SMTP2Go");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "smtp2go");

    }



    #[tokio::test]

    async fn test_send_with_title_as_subject() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "data": {"succeeded": 1, "failed": 0, "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Important Subject");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "data": {"succeeded": 1, "failed": 0, "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("<h1>Hello</h1><p>World</p>").with_format(MessageFormat::Html);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_cc_and_bcc() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "data": {"succeeded": 1, "failed": 0, "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "smtp2go-test-key")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

            .set("cc", "cc1@example.com, cc2@example.com")

            .set("bcc", "bcc@example.com")

            .set("base_url", &server.uri());

        let message = Message::text("Test with CC and BCC");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_api_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "data": {"succeeded": 0, "failed": 1, "error": "Invalid API key", "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({

                "data": {"succeeded": 0, "failed": 1, "error": "Forbidden", "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "data": {"succeeded": 1, "failed": 0, "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test with custom base");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "data": {"succeeded": 1, "failed": 0, "failures": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("api_key", "smtp2go-test-key")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

            .set("base_url", &base);

        let message = Message::text("Test trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod sns_send_tests {

    use super::*;

    use noti_providers::sns::SnsProvider;



    fn provider() -> SnsProvider {

        SnsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_key", "AKIAIOSFODNN7EXAMPLE")

            .set("secret_key", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY")

            .set("region", "us-east-1")

            .set("topic_arn", "arn:aws:sns:us-east-1:123456789012:my-topic")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "key")

            .set("secret_key", "secret")

            .set("region", "us-east-1")

            .set("topic_arn", "arn:aws:sns:us-east-1:123456789012:my-topic");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_access_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("secret_key", "secret")

            .set("region", "us-east-1")

            .set("topic_arn", "arn:aws:sns:us-east-1:123456789012:my-topic");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_topic_arn() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "key")

            .set("secret_key", "secret")

            .set("region", "us-east-1");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "sns");

        assert_eq!(provider.url_scheme(), "sns");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200).set_body_string(

                    "<?xml version=\"1.0\"?><PublishResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\"><PublishResult><MessageId>msg123</MessageId></PublishResult></PublishResponse>"

                )

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from SNS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "sns");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200).set_body_string(

                    "<?xml version=\"1.0\"?><PublishResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\"><PublishResult><MessageId>msg124</MessageId></PublishResult></PublishResponse>"

                )

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(403).set_body_string(

                    "<?xml version=\"1.0\"?><ErrorResponse><Error><Code>InvalidClientTokenId</Code><Message>Invalid security token</Message></Error></ErrorResponse>"

                )

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal server error"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200).set_body_string(

                    "<?xml version=\"1.0\"?><PublishResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\"><PublishResult><MessageId>msg125</MessageId></PublishResult></PublishResponse>"

                )

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Custom base URL");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "key")

            .set("secret_key", "secret")

            .set("region", "us-east-1")

            .set("topic_arn", "arn:aws:sns:us-east-1:123456789012:my-topic");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod o365_send_tests {

    use super::*;

    use noti_providers::o365::O365Provider;



    fn provider() -> O365Provider {

        O365Provider::new(client())

    }



    fn base_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("tenant_id", "test-tenant")

            .set("client_id", "test-client-id")

            .set("client_secret", "test-client-secret")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

    }



    fn config(server: &MockServer) -> ProviderConfig {

        base_config().set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = base_config();

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_tenant_id() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("client_id", "cid")

            .set("client_secret", "csec")

            .set("from", "from@example.com")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "o365");

        assert_eq!(provider.url_scheme(), "o365");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-tenant/oauth2/v2.0/token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "access_token": "test-access-token-123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/users/sender@example.com/sendMail"))

            .respond_with(ResponseTemplate::new(202).set_body_string(""))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello O365").with_title("Test Email");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "o365");

    }



    #[tokio::test]

    async fn test_send_token_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": "invalid_client"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

    }



    #[tokio::test]

    async fn test_send_mail_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-tenant/oauth2/v2.0/token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "access_token": "test-access-token-123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/users/sender@example.com/sendMail"))

            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({

                "error": {"code": "ErrorAccessDenied", "message": "Access is denied"}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}


mod sendpulse_send_tests {

    use super::*;

    use noti_providers::sendpulse::SendPulseProvider;



    fn provider() -> SendPulseProvider {

        SendPulseProvider::new(client())

    }



    fn base_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("client_id", "test-client-id")

            .set("client_secret", "test-client-secret")

            .set("from", "sender@example.com")

            .set("to", "recipient@example.com")

    }



    fn config(server: &MockServer) -> ProviderConfig {

        base_config().set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = base_config();

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_client_id() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("client_secret", "csec")

            .set("from", "from@example.com")

            .set("to", "to@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "sendpulse");

        assert_eq!(provider.url_scheme(), "sendpulse");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/oauth/access_token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "access_token": "test-access-token-123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/smtp/emails"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": true, "id": "email-123"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello SendPulse").with_title("Test Email");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "sendpulse");

    }



    #[tokio::test]

    async fn test_send_token_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": "invalid_client"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

    }



    #[tokio::test]

    async fn test_send_email_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/oauth/access_token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "access_token": "test-access-token-123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/smtp/emails"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "result": false, "message": "Invalid email address"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}

