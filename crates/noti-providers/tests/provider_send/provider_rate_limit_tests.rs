//! Provider send tests: rate_limit category.

use super::provider_test_utils::*;

mod rate_limit_tests {

    use noti_core::{Message, NotiError, NotifyProvider, ProviderConfig};

    use noti_providers::discord::DiscordProvider;

    use noti_providers::slack::SlackProvider;

    use noti_providers::telegram::TelegramProvider;

    use wiremock::matchers::method;

    use wiremock::{Mock, MockServer, ResponseTemplate};



    fn slack_config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("webhook_url", format!("{}/webhook", server.uri()))

    }



    fn telegram_config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("bot_token", "123456:ABC")

            .set("chat_id", "-1001234567890")

            .set("base_url", server.uri())

    }



    fn discord_config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("webhook_id", "123456")

            .set("webhook_token", "abc-def")

            .set("base_url", server.uri())

    }



    // ── Slack rate-limit tests ──



    #[tokio::test]

    async fn test_slack_rate_limited_429_with_retry_after_header() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(429)

                    .insert_header("retry-after", "30")

                    .set_body_string("rate limited"),

            )

            .mount(&server)

            .await;



        let provider = SlackProvider::new(reqwest::Client::new());

        let config = slack_config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.is_rate_limited(), "expected RateLimited error, got: {err}");

        if let NotiError::RateLimited {

            retry_after_secs, ..

        } = err

        {

            assert_eq!(retry_after_secs, Some(30));

        }

    }



    #[tokio::test]

    async fn test_slack_rate_limited_429_without_retry_after() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))

            .mount(&server)

            .await;



        let provider = SlackProvider::new(reqwest::Client::new());

        let config = slack_config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        assert!(result.unwrap_err().is_rate_limited());

    }



    // ── Telegram rate-limit tests ──



    #[tokio::test]

    async fn test_telegram_rate_limited_429_with_retry_after() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(429)

                    .insert_header("retry-after", "60")

                    .set_body_json(serde_json::json!({

                        "ok": false,

                        "description": "Too Many Requests: retry after 60",

                        "error_code": 429,

                        "parameters": {"retry_after": 60}

                    })),

            )

            .mount(&server)

            .await;



        let provider = TelegramProvider::new(reqwest::Client::new());

        let config = telegram_config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.is_rate_limited(), "expected RateLimited error, got: {err}");

        if let NotiError::RateLimited {

            retry_after_secs, ..

        } = err

        {

            assert_eq!(retry_after_secs, Some(60));

        }

    }



    #[tokio::test]

    async fn test_telegram_rate_limited_429_without_header() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(429)

                    .set_body_json(serde_json::json!({

                        "ok": false,

                        "description": "Too Many Requests",

                        "error_code": 429

                    })),

            )

            .mount(&server)

            .await;



        let provider = TelegramProvider::new(reqwest::Client::new());

        let config = telegram_config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        assert!(result.unwrap_err().is_rate_limited());

    }



    // ── Discord rate-limit tests ──



    #[tokio::test]

    async fn test_discord_rate_limited_429_with_retry_after_in_body() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(429)

                    .set_body_json(serde_json::json!({

                        "message": "You are being rate limited.",

                        "retry_after": 5.5,

                        "global": false

                    })),

            )

            .mount(&server)

            .await;



        let provider = DiscordProvider::new(reqwest::Client::new());

        let config = discord_config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.is_rate_limited(), "expected RateLimited error, got: {err}");

        if let NotiError::RateLimited {

            retry_after_secs, ..

        } = err

        {

            // 5.5 as f64 → 5 as u64

            assert_eq!(retry_after_secs, Some(5));

        }

    }



    #[tokio::test]

    async fn test_discord_rate_limited_429_with_header_and_body() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(429)

                    .insert_header("retry-after", "10")

                    .set_body_json(serde_json::json!({

                        "message": "rate limited",

                        "retry_after": 10.0,

                        "global": true

                    })),

            )

            .mount(&server)

            .await;



        let provider = DiscordProvider::new(reqwest::Client::new());

        let config = discord_config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        assert!(result.unwrap_err().is_rate_limited());

    }

}


mod rate_limit_429_integrated_tests {

    use super::*;



    // --- WeCom 429 ---

    mod wecom_429 {

    use super::*;

        use noti_providers::wecom::WeComProvider;



        fn make_config(server: &MockServer) -> ProviderConfig {

            ProviderConfig::new()

                .set("key", "test-key")

                .set("base_url", server.uri())

        }



        #[tokio::test]

        async fn test_wecom_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "30")

                        .set_body_json(serde_json::json!({"errcode": 45091, "errmsg": "rate limited"})),

                )

                .mount(&server)

                .await;



            let provider = WeComProvider::new(Client::new());

            let config = make_config(&server);

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- Teams 429 ---

    mod teams_429 {

    use super::*;

        use noti_providers::teams::TeamsProvider;



        #[tokio::test]

        async fn test_teams_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "15")

                        .set_body_string("Rate limited"),

                )

                .mount(&server)

                .await;



            let provider = TeamsProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("webhook_url", format!("{}/api/test", server.uri()));

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- Ntfy 429 ---

    mod ntfy_429 {

    use super::*;

        use noti_providers::ntfy::NtfyProvider;



        #[tokio::test]

        async fn test_ntfy_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "20")

                        .set_body_json(serde_json::json!({"code": 42901, "error": "rate limited"})),

                )

                .mount(&server)

                .await;



            let provider = NtfyProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("topic", "test")

                .set("server", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- SendGrid 429 ---

    mod sendgrid_429 {

    use super::*;

        use noti_providers::sendgrid::SendGridProvider;



        #[tokio::test]

        async fn test_sendgrid_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "300")

                        .set_body_json(serde_json::json!({"errors": [{"message": "rate limit exceeded"}]})),

                )

                .mount(&server)

                .await;



            let provider = SendGridProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("api_key", "SG.test")

                .set("from", "f@d.com")

                .set("to", "t@d.com")

                .set("base_url", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- Opsgenie 429 ---

    mod opsgenie_429 {

    use super::*;

        use noti_providers::opsgenie::OpsgenieProvider;



        #[tokio::test]

        async fn test_opsgenie_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "5")

                        .set_body_json(serde_json::json!({"message": "rate limit exceeded", "took": 0.001})),

                )

                .mount(&server)

                .await;



            let provider = OpsgenieProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("api_key", "test-key")

                .set("base_url", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- PagerDuty 429 ---

    mod pagerduty_429 {

    use super::*;

        use noti_providers::pagerduty::PagerDutyProvider;



        #[tokio::test]

        async fn test_pagerduty_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "10")

                        .set_body_json(serde_json::json!({"status": "error", "message": "rate limited"})),

                )

                .mount(&server)

                .await;



            let provider = PagerDutyProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("integration_key", "test-key")

                .set("base_url", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- Mailgun 429 ---

    mod mailgun_429 {

    use super::*;

        use noti_providers::mailgun::MailgunProvider;



        #[tokio::test]

        async fn test_mailgun_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "30")

                        .set_body_json(serde_json::json!({"message": "rate limit exceeded"})),

                )

                .mount(&server)

                .await;



            let provider = MailgunProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("api_key", "test-key")

                .set("domain", "test.com")

                .set("to", "t@test.com")

                .set("base_url", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- SparkPost 429 ---

    mod sparkpost_429 {

    use super::*;

        use noti_providers::sparkpost::SparkPostProvider;



        #[tokio::test]

        async fn test_sparkpost_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "60")

                        .set_body_json(serde_json::json!({"errors": [{"message": "limit exceeded"}]})),

                )

                .mount(&server)

                .await;



            let provider = SparkPostProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("api_key", "test-key")

                .set("from", "f@d.com")

                .set("to", "t@d.com")

                .set("base_url", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- Vonage SMS 429 ---

    mod vonage_429 {

    use super::*;

        use noti_providers::vonage::VonageProvider;



        #[tokio::test]

        async fn test_vonage_sms_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .insert_header("retry-after", "10")

                        .set_body_json(serde_json::json!({"messages": [{"status": "6", "error-text": "rate limited"}]})),

                )

                .mount(&server)

                .await;



            let provider = VonageProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("api_key", "k")

                .set("api_secret", "s")

                .set("from", "+1")

                .set("to", "+2")

                .set("base_url", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }



    // --- SES 429 ---

    mod ses_429 {

    use super::*;

        use noti_providers::ses::SesProvider;



        #[tokio::test]

        async fn test_ses_429_with_retry_after() {

            let server = MockServer::start().await;

            Mock::given(method("POST"))

                .respond_with(

                    ResponseTemplate::new(429)

                        .set_body_string("<ErrorResponse><Error><Code>Throttling</Code></Error></ErrorResponse>"),

                )

                .mount(&server)

                .await;



            let provider = SesProvider::new(Client::new());

            let config = ProviderConfig::new()

                .set("access_key", "k")

                .set("secret_key", "s")

                .set("region", "us-east-1")

                .set("from", "f@d.com")

                .set("to", "t@d.com")

                .set("base_url", server.uri());

            let result = provider.send(&Message::text("hi"), &config).await;

            assert!(result.is_err());

            assert!(result.unwrap_err().is_rate_limited());

        }

    }

}

