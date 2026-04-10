//! Provider send tests: media category.

use super::provider_test_utils::*;

mod lunasea_send_tests {

    use super::*;

    use noti_providers::lunasea::LunaseaProvider;



    fn provider() -> LunaseaProvider {

        LunaseaProvider::new(client())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_user_token() {

        let provider = provider();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "lunasea");

        assert_eq!(provider.url_scheme(), "lunasea");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "user_token" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("base_url", &server.uri());

        let message = Message::text("Hello LunaSea");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "lunasea");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("base_url", &server.uri());

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_target_device() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("target", "device")

            .set("base_url", &server.uri());

        let message = Message::text("Device notification");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_image_config() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("image", "https://example.com/image.png")

            .set("base_url", &server.uri());

        let message = Message::text("With image");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error_message() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "message": "Invalid token"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "bad-token")

            .set("base_url", &server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("base_url", &server.uri());

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

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("base_url", &server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

        assert_eq!(result.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("base_url", &server.uri());

        let message = Message::text("Custom base");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user_token", "test-token");

        // Validate that config works without base_url

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))

            .mount(&server)

            .await;



        let provider = provider();

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("user_token", "test-token")

            .set("base_url", &base);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod nextcloud_send_tests {

    use super::*;

    use noti_providers::nextcloud::NextcloudProvider;

    use wiremock::matchers::path_regex;



    fn provider() -> NextcloudProvider {

        NextcloudProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("user", "admin")

            .set("password", "test-password")

            .set("host", "cloud.example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user", "admin")

            .set("password", "pass")

            .set("host", "cloud.example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_user() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("password", "pass")

            .set("host", "cloud.example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user", "admin")

            .set("password", "pass");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "nextcloud");

        assert_eq!(provider.url_scheme(), "ncloud");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path_regex(r"/ocs/v2\.php/apps/admin_notifications/api/v1/notifications/userToNotify/admin"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 200 }, "data": {} }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Nextcloud");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "nextcloud");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path_regex(r"/ocs/v2\.php/apps/admin_notifications/api/v1/notifications/userToNotify/admin"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 200 }, "data": {} }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_target_user() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path_regex(r"/ocs/v2\.php/apps/admin_notifications/api/v1/notifications/userToNotify/john"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 200 }, "data": {} }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("target_user", "john");

        let message = Message::text("Targeted notification");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path_regex(r"/ocs/v2\.php/apps/admin_notifications/api/v1/notifications/userToNotify/admin"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 401, "message": "Unauthorized" } }

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 200 }, "data": {} }

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

        let config = ProviderConfig::new()

            .set("user", "admin")

            .set("password", "pass")

            .set("host", "cloud.example.com");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod emby_send_tests {

    use super::*;

    use noti_providers::emby::EmbyProvider;



    fn provider() -> EmbyProvider {

        EmbyProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("host", "localhost:8096")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("host", "localhost:8096");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "localhost:8096");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "emby");

        assert_eq!(provider.url_scheme(), "emby");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/Notifications/Admin"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Emby");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "emby");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_to_specific_user() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/Notifications/user123"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("user_id", "user123");

        let message = Message::text("User notification");



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

            .respond_with(ResponseTemplate::new(204))

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

            .set("api_key", "key")

            .set("host", "localhost:8096");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("host", "localhost:8096")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod jellyfin_send_tests {

    use super::*;

    use noti_providers::jellyfin::JellyfinProvider;



    fn provider() -> JellyfinProvider {

        JellyfinProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("host", "localhost:8096")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("host", "localhost:8096");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "localhost:8096");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-api-key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "jellyfin");

        assert_eq!(provider.url_scheme(), "jellyfin");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Jellyfin");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "jellyfin");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

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

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "Unauthorized"})))

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

            .respond_with(ResponseTemplate::new(204))

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

            .set("host", "localhost:8096");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("host", "localhost:8096")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod synology_send_tests {

    use super::*;

    use noti_providers::synology::SynologyProvider;



    fn provider() -> SynologyProvider {

        SynologyProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "nas.local")

            .set("token", "test-token")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "nas.local")

            .set("token", "tok");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "tok");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("host", "nas.local");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "synology");

        assert_eq!(provider.url_scheme(), "synology");

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

                "success": true

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Synology");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "synology");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true

            })))

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

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": {"code": 400}, "success": false

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

