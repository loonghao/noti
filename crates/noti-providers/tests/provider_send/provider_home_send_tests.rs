//! Provider send tests: home category.

use super::provider_test_utils::*;

mod homeassistant_send_tests {

    use super::*;

    use noti_providers::homeassistant::HomeAssistantProvider;



    fn provider() -> HomeAssistantProvider {

        HomeAssistantProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("host", "homeassistant.local:8123")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "token")

            .set("host", "homeassistant.local:8123");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "homeassistant.local:8123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new().set("access_token", "token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "homeassistant");

        assert_eq!(provider.url_scheme(), "hassio");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/services/notify/notify"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([

                { "entity_id": "notify.notify", "state": "ok" }

            ])))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Home Assistant");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "homeassistant");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/services/notify/notify"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([

                { "entity_id": "notify.notify", "state": "ok" }

            ])))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_custom_target() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/services/notify/mobile_app_phone"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([

                { "entity_id": "notify.mobile_app_phone", "state": "ok" }

            ])))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("target", "notify.mobile_app_phone");

        let message = Message::text("Mobile notification");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/services/notify/notify"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "message": "Invalid access token"

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

            .and(path("/api/services/notify/notify"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "message": "Internal server error"

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([

                { "entity_id": "notify.notify", "state": "ok" }

            ])))

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

            .set("access_token", "token")

            .set("host", "homeassistant.local:8123");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([

                { "entity_id": "notify.notify", "state": "ok" }

            ])))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("host", "homeassistant.local:8123")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod lametric_send_tests {

    use super::*;

    use noti_providers::lametric::LaMetricProvider;



    fn provider() -> LaMetricProvider {

        LaMetricProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("host", "192.168.1.100")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-key")

            .set("host", "192.168.1.100");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "192.168.1.100");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "lametric");

        assert_eq!(provider.url_scheme(), "lametric");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/device/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello LaMetric");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "lametric");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/device/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_priority_and_sound() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/device/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("priority", "warning")

            .set("sound", "alarm");

        let message = Message::text("Warning!");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/device/notifications"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "errors": [{ "message": "Invalid API key" }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

        assert_eq!(result.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

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

            .set("api_key", "test-key")

            .set("host", "192.168.1.100");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("host", "192.168.1.100")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod enigma2_send_tests {

    use super::*;

    use noti_providers::enigma2::Enigma2Provider;



    fn provider() -> Enigma2Provider {

        Enigma2Provider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "192.168.1.50")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "192.168.1.50");

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

        assert_eq!(provider.name(), "enigma2");

        assert_eq!(provider.url_scheme(), "enigma2");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .and(path("/api/message"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Enigma2");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "enigma2");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_timeout_and_type() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("timeout", "30")

            .set("msg_type", "2");

        let message = Message::text("Custom settings");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({"result": false, "message": "Error"})))

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

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({"message": "Server error"})))

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

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

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

        let config = ProviderConfig::new().set("host", "192.168.1.50");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "192.168.1.50")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod growl_send_tests {

    use super::*;

    use noti_providers::growl::GrowlProvider;



    fn provider() -> GrowlProvider {

        GrowlProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "192.168.1.100");

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

        assert_eq!(provider.name(), "growl");

        assert_eq!(provider.url_scheme(), "growl");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Growl");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "growl");

        assert_eq!(result.status_code, Some(200));

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

        let message = Message::text("Body text").with_title("Alert Title");



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

            .set("host", "192.168.1.100")

            .set("port", "23053");

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

            .set("host", "192.168.1.100")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod kodi_send_tests {

    use super::*;

    use noti_providers::kodi::KodiProvider;



    fn provider() -> KodiProvider {

        KodiProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "192.168.1.100");

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

        assert_eq!(provider.name(), "kodi");

        assert_eq!(provider.url_scheme(), "kodi");

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

                ResponseTemplate::new(200).set_body_json(serde_json::json!({

                    "jsonrpc": "2.0",

                    "result": "OK",

                    "id": 1

                }))

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Kodi");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "kodi");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200).set_body_json(serde_json::json!({

                    "jsonrpc": "2.0",

                    "result": "OK",

                    "id": 1

                }))

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

                ResponseTemplate::new(500).set_body_json(serde_json::json!({"error": "Internal error"}))

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

                ResponseTemplate::new(200).set_body_json(serde_json::json!({

                    "jsonrpc": "2.0",

                    "result": "OK",

                    "id": 1

                }))

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

            .set("host", "192.168.1.100")

            .set("port", "8080");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200).set_body_json(serde_json::json!({

                    "jsonrpc": "2.0",

                    "result": "OK",

                    "id": 1

                }))

            )

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}

