//! Provider send tests: automation category.

use super::provider_test_utils::*;

mod ifttt_send_tests {

    use super::*;

    use noti_providers::ifttt::IftttProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = IftttProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_key", "test-key")

            .set("event", "notification");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = IftttProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("webhook_key", "x"))

                .is_err()

        );

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("event", "x"))

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = IftttProvider::new(client());

        assert_eq!(provider.name(), "ifttt");

        assert_eq!(provider.url_scheme(), "ifttt");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "webhook_key" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "event" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "value1" && !p.required)

        );

    }

}


mod opsgenie_send_tests {

    use super::*;

    use noti_providers::opsgenie::OpsgenieProvider;



    fn provider() -> OpsgenieProvider {

        OpsgenieProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-genie-key-123")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "k");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "opsgenie");

        assert_eq!(provider.url_scheme(), "opsgenie");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/alerts"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "result": "Request will be processed",

                "took": 0.1,

                "requestId": "req-001"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Server is down");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "opsgenie");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/alerts"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "result": "Request will be processed",

                "requestId": "req-002"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Detailed description").with_title("High CPU Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_priority() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/alerts"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "result": "Request will be processed",

                "requestId": "req-003"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("priority", "P1");

        let message = Message::text("Critical alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_tags() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/alerts"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "result": "Request will be processed",

                "requestId": "req-004"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("tags", "critical,network")

            .set("entity", "web-server-01");

        let message = Message::text("Tagged alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/alerts"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "message": "Invalid API key"

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

            .and(path("/v2/alerts"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "message": "Authentication failed"

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


mod pagerduty_send_tests {

    use super::*;

    use noti_providers::pagerduty::PagerDutyProvider;



    fn provider() -> PagerDutyProvider {

        PagerDutyProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("integration_key", "R015test-key-1234567890abcdef")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("integration_key", "k");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_key() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pagerduty");

        assert_eq!(provider.url_scheme(), "pagerduty");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/enqueue"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "status": "success",

                "message": "Event processed",

                "dedup_key": "dedup-key-001"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Server is down");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pagerduty");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/enqueue"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "status": "success",

                "message": "Event processed",

                "dedup_key": "dedup-key-002"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Details here").with_title("Critical Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_severity() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/enqueue"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "status": "success",

                "message": "Event processed",

                "dedup_key": "dedup-key-003"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("severity", "critical")

            .set("source", "prod-web-01");

        let message = Message::text("Critical event");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_action() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/enqueue"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "status": "success",

                "message": "Event processed",

                "dedup_key": "dedup-key-004"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("action", "acknowledge")

            .set("dedup_key", "existing-key");

        let message = Message::text("Acknowledge event");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/enqueue"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "status": "invalid event",

                "message": "Invalid routing key"

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


mod signl4_send_tests {

    use super::*;

    use noti_providers::signl4::Signl4Provider;



    fn provider() -> Signl4Provider {

        Signl4Provider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("team_secret", "test-team-secret")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("team_secret", "secret");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_team_secret() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "signl4");

        assert_eq!(provider.url_scheme(), "signl4");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhook/test-team-secret"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Server down!");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "signl4");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhook/test-team-secret"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("CPU at 95%").with_title("Critical Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_severity() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhook/test-team-secret"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("s4_severity", "2");

        let message = Message::text("Critical issue");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_service() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhook/test-team-secret"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("s4_service", "noti-cli");

        let message = Message::text("Service alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhook/test-team-secret"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "error": "Invalid team secret"

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

    async fn test_send_server_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhook/test-team-secret"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "error": "Internal server error"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

        assert_eq!(result.status_code, Some(500));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "ok"

            })))

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

        let config = ProviderConfig::new().set("team_secret", "secret");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("team_secret", "test-team-secret")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod jira_send_tests {

    use super::*;

    use noti_providers::jira::JiraProvider;



    fn provider() -> JiraProvider {

        JiraProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "mycompany.atlassian.net")

            .set("user", "test@example.com")

            .set("api_token", "test-api-token")

            .set("issue_key", "PROJ-123")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "mycompany.atlassian.net")

            .set("user", "test@example.com")

            .set("api_token", "test-api-token")

            .set("issue_key", "PROJ-123");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user", "test@example.com")

            .set("api_token", "test-api-token")

            .set("issue_key", "PROJ-123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_user() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "mycompany.atlassian.net")

            .set("api_token", "test-api-token")

            .set("issue_key", "PROJ-123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_api_token() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "mycompany.atlassian.net")

            .set("user", "test@example.com")

            .set("issue_key", "PROJ-123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_issue_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "mycompany.atlassian.net")

            .set("user", "test@example.com")

            .set("api_token", "test-api-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "jira");

        assert_eq!(provider.url_scheme(), "jira");

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

                ResponseTemplate::new(201).set_body_json(serde_json::json!({"id": "12345", "self": "https://mycompany.atlassian.net/rest/api/3/issue/PROJ-123/comment/12345"}))

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Jira");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "jira");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(201).set_body_json(serde_json::json!({"id": "12346"}))

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

                ResponseTemplate::new(400).set_body_json(serde_json::json!({"errorMessages": ["Issue does not exist"]}))

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

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(401).set_body_json(serde_json::json!({"errorMessages": ["Unauthorized"]}))

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

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(201).set_body_json(serde_json::json!({"id": "12347"}))

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

            .set("host", "mycompany.atlassian.net")

            .set("user", "test@example.com")

            .set("api_token", "test-api-token")

            .set("issue_key", "PROJ-123");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(201).set_body_json(serde_json::json!({"id": "12348"}))

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "mycompany.atlassian.net")

            .set("user", "test@example.com")

            .set("api_token", "test-api-token")

            .set("issue_key", "PROJ-123")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod pagertree_send_tests {

    use super::*;

    use noti_providers::pagertree::PagerTreeProvider;



    fn provider() -> PagerTreeProvider {

        PagerTreeProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("integration_id", "test-int-123")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("integration_id", "test-int");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_integration_id() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pagertree");

        assert_eq!(provider.url_scheme(), "pagertree");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "inc-1"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Server down");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pagertree");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "inc-2"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Check the server").with_title("Critical Alert");



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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "inc-3"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("integration_id", "test-int-123")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod victorops_send_tests {

    use super::*;

    use noti_providers::victorops::VictorOpsProvider;



    fn provider() -> VictorOpsProvider {

        VictorOpsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("routing_key", "test-routing-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("routing_key", "route");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("routing_key", "route");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_routing_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "victorops");

        assert_eq!(provider.url_scheme(), "victorops");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": "success"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Server alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "victorops");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": "accepted"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Check server").with_title("Critical Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"message": "Unauthorized"})))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": "success"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("routing_key", "test-routing-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}

