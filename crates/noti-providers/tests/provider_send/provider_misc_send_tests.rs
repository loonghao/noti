//! Provider send tests: misc category.

use super::provider_test_utils::*;

mod webpush_send_tests {

    use super::*;

    use noti_providers::webpush::WebPushProvider;



    fn provider() -> WebPushProvider {

        WebPushProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("endpoint", &server.uri())

            .set("p256dh", "BEl62iUYgUivxIkvdfMV3D-4l7xLqgH4R wQ8X7r6H8Xz7kT8cL9qM3nV5pX7aB2dE4fG6hJ8kL0mN2oP4qR6sT8uV0wX2yZ")

            .set("auth", "dGhpcyBpcyBhIHRlc3Q")

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("endpoint", "https://push.example.com/v1/abc")

            .set("p256dh", "key123")

            .set("auth", "auth123");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_endpoint() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("p256dh", "key123")

            .set("auth", "auth123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "webpush");

        assert_eq!(provider.url_scheme(), "webpush");

        assert!(provider.supports_attachments());

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(201))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from WebPush");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "webpush");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(201))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Push Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_ttl_and_urgency() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(header("TTL", "3600"))

            .and(header("Urgency", "high"))

            .respond_with(ResponseTemplate::new(201))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("ttl", "3600")

            .set("urgency", "high");

        let message = Message::text("Urgent push");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(410).set_body_string("Subscription expired"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_server_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_success_200() {

        let server = MockServer::start().await;

        // Some push services return 200 OK

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("200 OK test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod apprise_send_tests {

    use super::*;

    use noti_providers::apprise::AppriseProvider;



    fn provider() -> AppriseProvider {

        AppriseProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "http://localhost:8000")

            .set("urls", "slack://token_a/token_b/token_c")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "http://localhost:8000")

            .set("urls", "slack://token_a/token_b/token_c");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new().set("urls", "slack://token_a/token_b/token_c");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "apprise");

        assert_eq!(provider.url_scheme(), "apprise");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/notify"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Apprise");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "apprise");

    }



    #[tokio::test]

    async fn test_send_with_config_key() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/notify/my-config"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "http://localhost:8000")

            .set("config_key", "my-config")

            .set("base_url", &server.uri());

        let message = Message::text("Stateful notification");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_string("Bad request"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_server_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))

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

            .respond_with(ResponseTemplate::new(200))

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

            .set("host", "http://localhost:8000")

            .set("urls", "slack://token_a/token_b/token_c");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "http://localhost:8000")

            .set("urls", "slack://token_a/token_b/token_c")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod kumulos_send_tests {

    use super::*;

    use noti_providers::kumulos::KumulosProvider;



    fn provider() -> KumulosProvider {

        KumulosProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("server_key", "test-server-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("server_key", "test-server-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("server_key", "test-server-key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_server_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-api-key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "kumulos");

        assert_eq!(provider.url_scheme(), "kumulos");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-123"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Kumulos");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "kumulos");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-124"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_channel() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-125"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("server_key", "test-server-key")

            .set("channel", "test-channel")

            .set("base_url", &server.uri());

        let message = Message::text("Channel message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-126"})))

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

            .set("api_key", "test-api-key")

            .set("server_key", "test-server-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-127"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("server_key", "test-server-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod rsyslog_send_tests {

    use super::*;

    use noti_providers::rsyslog::RsyslogProvider;



    fn provider() -> RsyslogProvider {

        RsyslogProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "logs.example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "logs.example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "rsyslog");

        assert_eq!(provider.url_scheme(), "rsyslog");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Syslog message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "rsyslog");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_with_token() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "logs.example.com")

            .set("token", "my-auth-token")

            .set("base_url", &server.uri());

        let message = Message::text("With auth");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod reddit_send_tests {

    use super::*;

    use noti_providers::reddit::RedditProvider;



    fn provider() -> RedditProvider {

        RedditProvider::new(client())

    }



    fn base_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("client_id", "test-client-id")

            .set("client_secret", "test-client-secret")

            .set("user", "testuser")

            .set("password", "testpassword")

            .set("to", "targetuser")

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

            .set("user", "user")

            .set("password", "pass")

            .set("to", "target");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "reddit");

        assert_eq!(provider.url_scheme(), "reddit");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/access_token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "access_token": "test-access-token-123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/api/compose"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "json": {"errors": []}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Reddit").with_title("Test PM");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "reddit");

    }



    #[tokio::test]

    async fn test_send_token_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": "invalid_grant"

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

    async fn test_send_compose_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/access_token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "access_token": "test-access-token-123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/api/compose"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "json": {"errors": [["USER_DOESNT_EXIST", "that user doesn't exist", "to"]]}

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


mod twitter_send_tests {

    use super::*;

    use noti_providers::twitter::TwitterProvider;



    fn provider() -> TwitterProvider {

        TwitterProvider::new(client())

    }



    fn base_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("bearer_token", "test-bearer-token")

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

    async fn test_validate_missing_bearer_token() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "twitter");

        assert_eq!(provider.url_scheme(), "twitter");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_tweet_success() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/2/tweets"))

            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({

                "data": {"id": "tweet-123", "text": "Hello Twitter"}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Twitter");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "twitter");

    }



    #[tokio::test]

    async fn test_send_dm_success() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/2/dm_conversations/with/messages"))

            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({

                "data": {"id": "dm-123"}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("bearer_token", "test-bearer-token")

            .set("mode", "dm")

            .set("dm_user_id", "user-123")

            .set("base_url", &server.uri());

        let message = Message::text("Hello DM");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}


mod parse_send_tests {

    use super::*;

    use noti_providers::parse::ParseProvider;



    fn provider() -> ParseProvider {

        ParseProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("rest_api_key", "test-rest-api-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("app_id", "aid")

            .set("rest_api_key", "rak");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_app_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("rest_api_key", "rak");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_rest_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("app_id", "aid");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "parse");

        assert_eq!(provider.url_scheme(), "parse");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Parse");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "parse");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Push Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("rest_api_key", "test-rest-api-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod workflows_send_tests {

    use super::*;

    use noti_providers::workflows::WorkflowsProvider;



    fn provider() -> WorkflowsProvider {

        WorkflowsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "prod-XX.westus.logic.azure.com")

            .set("workflow", "test-workflow-id")

            .set("signature", "test-sig-value")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "host")

            .set("workflow", "wf")

            .set("signature", "sig");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("workflow", "wf")

            .set("signature", "sig");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_workflow() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "host")

            .set("signature", "sig");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_signature() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "host")

            .set("workflow", "wf");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "workflows");

        assert_eq!(provider.url_scheme(), "workflows");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(202).set_body_string(""))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Workflows");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "workflows");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(202).set_body_string(""))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": {"code": "Unauthorized", "message": "Invalid signature"}

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

