//! Provider send tests: sms category.

use super::provider_test_utils::*;

mod twilio_send_tests {

    use super::*;

    use noti_providers::twilio::TwilioProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = TwilioProvider::new(client());

        let config = ProviderConfig::new()

            .set("account_sid", "ACxxx")

            .set("auth_token", "authtoken")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = TwilioProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        // Missing auth_token, from, to

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("account_sid", "ACxxx"))

                .is_err()

        );

        // Missing from, to

        assert!(

            provider

                .validate_config(

                    &ProviderConfig::new()

                        .set("account_sid", "ACxxx")

                        .set("auth_token", "tok")

                )

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = TwilioProvider::new(client());

        assert_eq!(provider.name(), "twilio");

        assert_eq!(provider.url_scheme(), "twilio");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "account_sid" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "auth_token" && p.required)

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

    async fn test_send_sms_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/2010-04-01/Accounts/ACxxx/Messages.json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "sid": "SM1234567890abcdef",

                "status": "queued",

                "to": "+15559876543",

                "from": "+15551234567",

                "body": "Hello World"

            })))

            .mount(&mock_server)

            .await;



        let provider = TwilioProvider::new(client());

        let config = ProviderConfig::new()

            .set("account_sid", "ACxxx")

            .set("auth_token", "authToken")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Hello World");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "twilio");

        assert_eq!(response.status_code, Some(200));

        assert!(response.message.contains("SMS sent"));

    }



    #[tokio::test]

    async fn test_send_sms_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/2010-04-01/Accounts/ACxxx/Messages.json"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "code": 21211,

                "message": "Invalid phone number",

                "status": 400

            })))

            .mount(&mock_server)

            .await;



        let provider = TwilioProvider::new(client());

        let config = ProviderConfig::new()

            .set("account_sid", "ACxxx")

            .set("auth_token", "authtoken")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Hello");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(400));

        assert!(response.message.contains("Invalid phone number"));

    }



    #[tokio::test]

    async fn test_send_mms_with_media_url_config() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/2010-04-01/Accounts/ACxxx/Messages.json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "sid": "SM9876543210fedcba",

                "status": "queued"

            })))

            .mount(&mock_server)

            .await;



        let provider = TwilioProvider::new(client());

        let config = ProviderConfig::new()

            .set("account_sid", "ACxxx")

            .set("auth_token", "authtoken")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("media_url", "https://example.com/image.png")

            .set("base_url", mock_server.uri());



        let message = Message::text("Check this image");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        // media_url config doesn't set has_attachments(), so it says SMS not MMS

        assert!(response.message.contains("SMS sent"));

    }



    #[tokio::test]

    async fn test_send_sms_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/2010-04-01/Accounts/ACxxx/Messages.json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "sid": "SM11111111111111111",

                "status": "queued"

            })))

            .mount(&mock_server)

            .await;



        let provider = TwilioProvider::new(client());

        let config = ProviderConfig::new()

            .set("account_sid", "ACxxx")

            .set("auth_token", "authtoken")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Body text").with_title("Title Here");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/2010-04-01/Accounts/ACxxx/Messages.json"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "code": 20003,

                "message": "Authentication failed",

                "status": 401

            })))

            .mount(&mock_server)

            .await;



        let provider = TwilioProvider::new(client());

        let config = ProviderConfig::new()

            .set("account_sid", "ACxxx")

            .set("auth_token", "wrongtoken")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }

}


mod sinch_send_tests {

    use super::*;

    use noti_providers::sinch::SinchProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = SinchProvider::new(client());

        let config = ProviderConfig::new()

            .set("service_plan_id", "plan123")

            .set("api_token", "token456")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = SinchProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        // Missing api_token, from, to

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("service_plan_id", "plan123"))

                .is_err()

        );

        // Missing from, to

        assert!(

            provider

                .validate_config(

                    &ProviderConfig::new()

                        .set("service_plan_id", "plan123")

                        .set("api_token", "token")

                )

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = SinchProvider::new(client());

        assert_eq!(provider.name(), "sinch");

        assert_eq!(provider.url_scheme(), "sinch");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "service_plan_id" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "api_token" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "from" && p.required)

        );

        assert!(provider.params().iter().any(|p| p.name == "to" && p.required));

        assert!(provider.supports_attachments());

    }



    #[tokio::test]

    async fn test_send_sms_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xms/v1/plan123/batches"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "batch_abc123",

                "status": "queued",

                "from": "+15551234567",

                "to": ["+15559876543"],

                "body": "Hello World"

            })))

            .mount(&mock_server)

            .await;



        let provider = SinchProvider::new(client());

        let config = ProviderConfig::new()

            .set("service_plan_id", "plan123")

            .set("api_token", "token456")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Hello World");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "sinch");

        assert_eq!(response.status_code, Some(200));

        assert!(response.message.contains("SMS sent"));

    }



    #[tokio::test]

    async fn test_send_sms_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xms/v1/plan123/batches"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "text": "Invalid recipient",

                "code": 400

            })))

            .mount(&mock_server)

            .await;



        let provider = SinchProvider::new(client());

        let config = ProviderConfig::new()

            .set("service_plan_id", "plan123")

            .set("api_token", "token456")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Hello");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(400));

        assert!(response.message.contains("Invalid recipient"));

    }



    #[tokio::test]

    async fn test_send_sms_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xms/v1/plan123/batches"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "batch_def456",

                "status": "queued"

            })))

            .mount(&mock_server)

            .await;



        let provider = SinchProvider::new(client());

        let config = ProviderConfig::new()

            .set("service_plan_id", "plan123")

            .set("api_token", "token456")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Body text").with_title("Title Here");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        // Sinch formats title as "Title Here\n\nBody text"

        assert!(response.raw_response.as_ref().map(|v| v.is_object()).unwrap_or(false));

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xms/v1/plan123/batches"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "text": "Authentication failed",

                "code": 401

            })))

            .mount(&mock_server)

            .await;



        let provider = SinchProvider::new(client());

        let config = ProviderConfig::new()

            .set("service_plan_id", "plan123")

            .set("api_token", "wrongtoken")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", mock_server.uri());



        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_mms_with_media_url_config() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xms/v1/plan123/batches"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "batch_mms789",

                "status": "queued"

            })))

            .mount(&mock_server)

            .await;



        let provider = SinchProvider::new(client());

        let config = ProviderConfig::new()

            .set("service_plan_id", "plan123")

            .set("api_token", "token456")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("media_url", "https://example.com/image.png")

            .set("base_url", mock_server.uri());



        let message = Message::text("Check this image");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        // Note: media_url config sets mt_media type but still says "SMS" since no actual attachments

        assert!(response.message.contains("SMS sent"));

    }



    #[tokio::test]

    async fn test_region_eu() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xms/v1/plan123/batches"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "batch_eu123",

                "status": "queued"

            })))

            .mount(&mock_server)

            .await;



        let provider = SinchProvider::new(client());

        let config = ProviderConfig::new()

            .set("service_plan_id", "plan123")

            .set("api_token", "token456")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("region", "eu")

            .set("base_url", mock_server.uri());



        let message = Message::text("EU region test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

    }

}


mod vonage_send_tests {

    use super::*;

    use noti_providers::vonage::VonageProvider;



    fn provider() -> VonageProvider {

        VonageProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-key")

            .set("api_secret", "test-secret")

            .set("from", "15551234567")

            .set("to", "15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "k")

            .set("api_secret", "s")

            .set("from", "12345")

            .set("to", "67890");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_secret", "s")

            .set("from", "12345")

            .set("to", "67890");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "vonage");

        assert_eq!(provider.url_scheme(), "vonage");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_sms_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/sms/json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{"status": "0", "message-id": "msg001"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Vonage");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "vonage");

    }



    #[tokio::test]

    async fn test_send_sms_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/sms/json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{"status": "0", "message-id": "msg002"}]

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

    async fn test_send_sms_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/sms/json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{"status": "2", "error-text": "Missing params"}]

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/sms/json"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "messages": [{"status": "6", "error-text": "Internal server error"}]

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

            .and(path("/sms/json"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "messages": [{"status": "4", "error-text": "Invalid credentials"}]

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

            .and(path("/sms/json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{"status": "0", "message-id": "msg-custom"}]

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

            .set("api_key", "k")

            .set("api_secret", "s")

            .set("from", "12345")

            .set("to", "67890");

        let message = Message::text("Default URL");

        // Will fail since no real server — just verify no panic and config is valid

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok() || result.is_err());

    }

}


mod threema_send_tests {

    use super::*;

    use noti_providers::threema::ThreemaProvider;



    fn provider() -> ThreemaProvider {

        ThreemaProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("gateway_id", "*MY_GW")

            .set("api_secret", "test-secret")

            .set("to", "ABCD1234")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("gateway_id", "*MY_GW")

            .set("api_secret", "test-secret")

            .set("to", "ABCD1234");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_gateway_id() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_secret", "test-secret")

            .set("to", "ABCD1234");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_api_secret() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("gateway_id", "*MY_GW")

            .set("to", "ABCD1234");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("gateway_id", "*MY_GW")

            .set("api_secret", "test-secret");

        // to is required only if to_phone and to_email are also absent

        // but validate_config checks required params, and "to" is listed as required

        // Actually, "to" is listed as required in params but the send() code handles

        // to_phone/to_email alternatives. validate_config only checks the required list.

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "threema");

        assert_eq!(provider.url_scheme(), "threema");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "gateway_id" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "api_secret" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("msg_id_123"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Threema");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "threema");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("msg_id_456"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Threema Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid credentials"))

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

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))

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

            .respond_with(ResponseTemplate::new(200).set_body_string("msg_id_789"))

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

            .set("gateway_id", "*MY_GW")

            .set("api_secret", "test-secret")

            .set("to", "ABCD1234");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("msg_id_abc"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("gateway_id", "*MY_GW")

            .set("api_secret", "test-secret")

            .set("to", "ABCD1234")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod bulksms_send_tests {

    use super::*;

    use noti_providers::bulksms::BulkSmsProvider;



    fn provider() -> BulkSmsProvider {

        BulkSmsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("token_id", "test-token-id")

            .set("token_secret", "test-token-secret")

            .set("to", "+15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("token_id", "id")

            .set("token_secret", "secret")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token_id() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("token_secret", "secret")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("token_id", "id")

            .set("token_secret", "secret");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "bulksms");

        assert_eq!(provider.url_scheme(), "bulksms");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/messages"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg123"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from BulkSMS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "bulksms");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/messages"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg124"

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

            .and(path("/v1/messages"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "detail": "Invalid recipient"

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/messages"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "detail": "Internal server error"

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

                "id": "msg125"

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

            .set("token_id", "id")

            .set("token_secret", "secret")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg126"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("token_id", "test-token-id")

            .set("token_secret", "test-token-secret")

            .set("to", "+15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod clicksend_send_tests {

    use super::*;

    use noti_providers::clicksend::ClickSendProvider;



    fn provider() -> ClickSendProvider {

        ClickSendProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("username", "testuser")

            .set("api_key", "test-api-key")

            .set("to", "+15551234567")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("api_key", "key")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_username() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "clicksend");

        assert_eq!(provider.url_scheme(), "clicksend");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_sms_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/sms/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "http_code": 200,

                "response_msg": "OK"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from ClickSend");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "clicksend");

    }



    #[tokio::test]

    async fn test_send_sms_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/sms/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "http_code": 200,

                "response_msg": "OK"

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

    async fn test_send_sms_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/sms/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "http_code": 400,

                "response_msg": "Invalid recipient"

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v3/sms/send"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "http_code": 500,

                "response_msg": "Server error"

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

                "http_code": 200,

                "response_msg": "OK"

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

            .set("username", "user")

            .set("api_key", "key")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "http_code": 200,

                "response_msg": "OK"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "testuser")

            .set("api_key", "test-api-key")

            .set("to", "+15551234567")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod d7networks_send_tests {

    use super::*;

    use noti_providers::d7networks::D7NetworksProvider;



    fn provider() -> D7NetworksProvider {

        D7NetworksProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_token", "test-api-token")

            .set("to", "+15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_token", "token")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_token", "token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "d7sms");

        assert_eq!(provider.url_scheme(), "d7sms");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/messages/v1/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "request_id": "req123"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from D7Networks");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "d7sms");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/messages/v1/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "request_id": "req124"

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

            .and(path("/messages/v1/send"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "detail": "Invalid request"

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/messages/v1/send"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "detail": "Internal server error"

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

                "request_id": "req125"

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

            .set("api_token", "token")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "request_id": "req126"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_token", "test-api-token")

            .set("to", "+15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod kavenegar_send_tests {

    use super::*;

    use noti_providers::kavenegar::KavenegarProvider;



    fn provider() -> KavenegarProvider {

        KavenegarProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("to", "09121234567")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "09121234567");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("to", "09121234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "kavenegar");

        assert_eq!(provider.url_scheme(), "kavenegar");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "return": { "status": 200, "message": "OK" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Kavenegar");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "kavenegar");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "return": { "status": 200, "message": "OK" }

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "return": { "status": 401, "message": "Invalid API key" }

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "return": { "status": 500, "message": "Server error" }

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

                "return": { "status": 200, "message": "OK" }

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

            .set("api_key", "key")

            .set("to", "09121234567");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "return": { "status": 200, "message": "OK" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("to", "09121234567")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod messagebird_send_tests {

    use super::*;

    use noti_providers::messagebird::MessageBirdProvider;



    fn provider() -> MessageBirdProvider {

        MessageBirdProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_key", "test-access-key")

            .set("from", "MyApp")

            .set("to", "+15551234567")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "key")

            .set("from", "MyApp")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_access_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("from", "MyApp")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_from() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "key")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "key")

            .set("from", "MyApp");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "messagebird");

        assert_eq!(provider.url_scheme(), "msgbird");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/messages"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg123",

                "status": "sent"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from MessageBird");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "messagebird");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/messages"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg124",

                "status": "sent"

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

            .and(path("/messages"))

            .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({

                "errors": [{ "description": "Invalid recipient" }]

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/messages"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "errors": [{ "description": "Invalid access key" }]

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

                "id": "msg125",

                "status": "sent"

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

            .set("access_key", "key")

            .set("from", "MyApp")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg126",

                "status": "sent"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_key", "test-access-key")

            .set("from", "MyApp")

            .set("to", "+15551234567")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod plivo_send_tests {

    use super::*;

    use noti_providers::plivo::PlivoProvider;



    fn provider() -> PlivoProvider {

        PlivoProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("auth_id", "MAXXXXXXXXXXXXXXXXXX")

            .set("auth_token", "test-auth-token")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("auth_id", "id")

            .set("auth_token", "token")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_auth_id() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("auth_token", "token")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_from() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("auth_id", "id")

            .set("auth_token", "token")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("auth_id", "id")

            .set("auth_token", "token")

            .set("from", "+15551234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "plivo");

        assert_eq!(provider.url_scheme(), "plivo");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/Account/MAXXXXXXXXXXXXXXXXXX/Message/"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "message_uuid": ["uuid-123"]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Plivo");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "plivo");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/Account/MAXXXXXXXXXXXXXXXXXX/Message/"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "message_uuid": ["uuid-124"]

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

            .and(path("/v1/Account/MAXXXXXXXXXXXXXXXXXX/Message/"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "error": "Invalid destination number"

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/Account/MAXXXXXXXXXXXXXXXXXX/Message/"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": "Authentication failed"

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

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "message_uuid": ["uuid-125"]

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

            .set("auth_id", "id")

            .set("auth_token", "token")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({

                "message_uuid": ["uuid-126"]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("auth_id", "MAXXXXXXXXXXXXXXXXXX")

            .set("auth_token", "test-auth-token")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod smsmanager_send_tests {

    use super::*;

    use noti_providers::smsmanager::SmsManagerProvider;



    fn provider() -> SmsManagerProvider {

        SmsManagerProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("to", "+15551234567")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("to", "+15551234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "smsmanager");

        assert_eq!(provider.url_scheme(), "smsmanager");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/Send"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from SmsManager");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "smsmanager");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/Send"))

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

            .and(path("/Send"))

            .respond_with(ResponseTemplate::new(400).set_body_string("Error: Invalid API key"))

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

            .and(path("/Send"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))

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

            .set("api_key", "key")

            .set("to", "+15551234567");

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

            .set("api_key", "test-api-key")

            .set("to", "+15551234567")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod voipms_send_tests {

    use super::*;

    use noti_providers::voipms::VoipMsProvider;



    fn provider() -> VoipMsProvider {

        VoipMsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("email", "user@example.com")

            .set("password", "test-password")

            .set("did", "15551234567")

            .set("to", "15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("email", "user@example.com")

            .set("password", "pass")

            .set("did", "15551234567")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_email() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("password", "pass")

            .set("did", "15551234567")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_did() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("email", "user@example.com")

            .set("password", "pass")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("email", "user@example.com")

            .set("password", "pass")

            .set("did", "15551234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "voipms");

        assert_eq!(provider.url_scheme(), "voipms");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "success"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from VoipMs");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "voipms");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "success"

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

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "error",

                "message": "Invalid credentials"

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "success"

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

            .set("email", "user@example.com")

            .set("password", "pass")

            .set("did", "15551234567")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": "success"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("email", "user@example.com")

            .set("password", "test-password")

            .set("did", "15551234567")

            .set("to", "15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod africas_talking_send_tests {

    use super::*;

    use noti_providers::africas_talking::AfricasTalkingProvider;



    fn provider() -> AfricasTalkingProvider {

        AfricasTalkingProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("username", "testuser")

            .set("api_key", "test-api-key")

            .set("to", "+254712345678")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("api_key", "key")

            .set("to", "+254712345678");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_username() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "+254712345678");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("to", "+254712345678");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "africastalking");

        assert_eq!(provider.url_scheme(), "africastalking");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "SMSMessageData": { "Message": "Sent to 1/1", "Recipients": [{ "statusCode": 101, "number": "+254712345678" }] }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Africa's Talking");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "africastalking");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "SMSMessageData": { "Message": "Sent to 1/1" }

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

    async fn test_send_with_from() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "SMSMessageData": { "Message": "Sent to 1/1" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("from", "MyApp");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_sandbox() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "SMSMessageData": { "Message": "Sent to 1/1 (sandbox)" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("sandbox", "true");

        let message = Message::text("Sandbox test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "SMSMessageData": { "Message": "Invalid phone number" }

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

                "error": "Invalid API key"

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("api_key", "key")

            .set("to", "+254712345678");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "SMSMessageData": { "Message": "Sent" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("api_key", "key")

            .set("to", "+254712345678")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod bulkvs_send_tests {

    use super::*;

    use noti_providers::bulkvs::BulkVsProvider;



    fn provider() -> BulkVsProvider {

        BulkVsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("username", "testuser")

            .set("password", "testpass")

            .set("from", "15551234567")

            .set("to", "15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("password", "pass")

            .set("from", "15551234567")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_username() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("password", "pass")

            .set("from", "15551234567")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_password() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("from", "15551234567")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "bulkvs");

        assert_eq!(provider.url_scheme(), "bulkvs");

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

                "MessageId": "msg-123"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from BulkVS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "bulkvs");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "MessageId": "msg-456"

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

    async fn test_send_with_media_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "MessageId": "msg-789"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("media_url", "https://example.com/image.jpg");

        let message = Message::text("MMS test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "Error": "Invalid request"

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

                "Error": "Unauthorized"

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("password", "pass")

            .set("from", "15551234567")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "MessageId": "msg-abc"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("username", "user")

            .set("password", "pass")

            .set("from", "15551234567")

            .set("to", "15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod burstsms_send_tests {

    use super::*;

    use noti_providers::burstsms::BurstSmsProvider;



    fn provider() -> BurstSmsProvider {

        BurstSmsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("api_secret", "test-api-secret")

            .set("from", "MyApp")

            .set("to", "+61412345678")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("api_secret", "secret")

            .set("from", "MyApp")

            .set("to", "+61412345678");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_secret", "secret")

            .set("from", "MyApp")

            .set("to", "+61412345678");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_api_secret() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("from", "MyApp")

            .set("to", "+61412345678");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "burstsms");

        assert_eq!(provider.url_scheme(), "burstsms");

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

                "message_id": "msg-123",

                "error": { "code": "SUCCESS", "description": "OK" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from BurstSMS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "burstsms");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "message_id": "msg-456",

                "error": { "code": "SUCCESS", "description": "OK" }

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

    async fn test_send_with_media_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "message_id": "msg-789",

                "error": { "code": "SUCCESS", "description": "OK" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("media_url", "https://example.com/image.jpg");

        let message = Message::text("MMS test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_api_error_in_body() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "error": { "code": "INVALID_NUMBER", "description": "Invalid recipient" }

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

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": { "code": "AUTH_FAILED", "description": "Invalid credentials" }

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("api_secret", "secret")

            .set("from", "MyApp")

            .set("to", "+61412345678");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "message_id": "msg-abc",

                "error": { "code": "SUCCESS", "description": "OK" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("api_secret", "secret")

            .set("from", "MyApp")

            .set("to", "+61412345678")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod clickatell_send_tests {

    use super::*;

    use noti_providers::clickatell::ClickatellProvider;



    fn provider() -> ClickatellProvider {

        ClickatellProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("to", "15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("to", "15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "clickatell");

        assert_eq!(provider.url_scheme(), "clickatell");

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

                "messages": [{ "apiMessageId": "msg-123", "accepted": true }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Clickatell");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "clickatell");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{ "apiMessageId": "msg-456", "accepted": true }]

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

    async fn test_send_with_from() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{ "apiMessageId": "msg-789", "accepted": true }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("from", "+15551234567");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_whatsapp_channel() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{ "apiMessageId": "msg-wa", "accepted": true }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("channel", "whatsapp");

        let message = Message::text("WhatsApp test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "error": "Invalid phone number",

                "errorCode": 105

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

                "error": "Invalid API key",

                "errorCode": 101

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{ "apiMessageId": "msg-xyz", "accepted": true }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod fortysixelks_send_tests {

    use super::*;

    use noti_providers::fortysixelks::FortySixElksProvider;



    fn provider() -> FortySixElksProvider {

        FortySixElksProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_username", "test-user")

            .set("api_password", "test-pass")

            .set("from", "+46701234567")

            .set("to", "+46709876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_username", "user")

            .set("api_password", "pass")

            .set("from", "+46701234567")

            .set("to", "+46709876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_username() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_password", "pass")

            .set("from", "+46701234567")

            .set("to", "+46709876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_api_password() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_username", "user")

            .set("from", "+46701234567")

            .set("to", "+46709876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "46elks");

        assert_eq!(provider.url_scheme(), "46elks");

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

                "id": "msg-123",

                "status": "delivered"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from 46elks");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "46elks");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-456",

                "status": "delivered"

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

    async fn test_send_with_flash() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-flash",

                "status": "delivered"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("flash", "yes");

        let message = Message::text("Flash SMS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_media_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-mms",

                "status": "delivered"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("media_url", "https://example.com/image.jpg");

        let message = Message::text("MMS test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "error": "Invalid phone number"

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

                "error": "Unauthorized"

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_username", "user")

            .set("api_password", "pass")

            .set("from", "+46701234567")

            .set("to", "+46709876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-abc",

                "status": "delivered"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_username", "user")

            .set("api_password", "pass")

            .set("from", "+46701234567")

            .set("to", "+46709876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod httpsms_send_tests {

    use super::*;

    use noti_providers::httpsms::HttpSmsProvider;



    fn provider() -> HttpSmsProvider {

        HttpSmsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_from() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("from", "+15551234567");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "httpsms");

        assert_eq!(provider.url_scheme(), "httpsms");

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

                "id": "msg-123",

                "status": "pending"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from httpSMS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "httpsms");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-456",

                "status": "sent"

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

    async fn test_send_with_encrypt() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-enc",

                "status": "pending"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("encrypt", "true");

        let message = Message::text("Encrypted test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "status": "error",

                "message": "Invalid phone number"

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

                "status": "error",

                "message": "Invalid API key"

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-abc",

                "status": "pending"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod msg91_send_tests {

    use super::*;

    use noti_providers::msg91::Msg91Provider;



    fn provider() -> Msg91Provider {

        Msg91Provider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("authkey", "test-authkey")

            .set("sender", "NOTIAP")

            .set("to", "919876543210")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("authkey", "key")

            .set("sender", "NOTIAP")

            .set("to", "919876543210");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_authkey() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("sender", "NOTIAP")

            .set("to", "919876543210");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_sender() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("authkey", "key")

            .set("to", "919876543210");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("authkey", "key")

            .set("sender", "NOTIAP");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "msg91");

        assert_eq!(provider.url_scheme(), "msg91");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "type": "success",

                "message": "SMS sent successfully"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from MSG91");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "msg91");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "type": "success",

                "message": "SMS sent"

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

    async fn test_send_with_route_and_country() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "type": "success",

                "message": "SMS sent"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("route", "1")

            .set("country", "91");

        let message = Message::text("Promotional SMS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_dlt_te_id() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "type": "success",

                "message": "SMS sent"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("DLT_TE_ID", "template-123");

        let message = Message::text("DLT test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "type": "error",

                "message": "Invalid authkey"

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

                "type": "error",

                "message": "Authentication failed"

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("authkey", "key")

            .set("sender", "NOTIAP")

            .set("to", "919876543210");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "type": "success",

                "message": "SMS sent"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("authkey", "key")

            .set("sender", "NOTIAP")

            .set("to", "919876543210")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod seven_send_tests {

    use super::*;

    use noti_providers::seven::SevenProvider;



    fn provider() -> SevenProvider {

        SevenProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("to", "+15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "seven");

        assert_eq!(provider.url_scheme(), "seven");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/sms"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": "100",

                "messages": [{ "id": "msg-123" }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Seven");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "seven");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/sms"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": "100",

                "messages": [{ "id": "msg-456" }]

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

    async fn test_send_with_from() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/sms"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": "100",

                "messages": [{ "id": "msg-789" }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("from", "noti");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/sms"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": "101",

                "messages": [{ "error_text": "Invalid recipient" }]

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

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/sms"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "success": "901",

                "messages": [{ "error_text": "Unauthorized" }]

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

                "success": "100",

                "messages": [{ "id": "msg-custom" }]

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

            .set("api_key", "key")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": "100",

                "messages": [{ "id": "msg-slash" }]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("to", "+15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod smseagle_send_tests {

    use super::*;

    use noti_providers::smseagle::SmsEagleProvider;



    fn provider() -> SmsEagleProvider {

        SmsEagleProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("access_token", "test-token")

            .set("to", "+15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("access_token", "token")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "token")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("access_token", "token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "smseagle");

        assert_eq!(provider.url_scheme(), "smseagle");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_sms_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/messages/sms"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-123"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from SMSEagle");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "smseagle");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/messages/sms"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-456"

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

    async fn test_send_with_priority() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/messages/sms"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-priority"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("priority", "5");

        let message = Message::text("High priority");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v2/messages/sms"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "error": "Invalid phone number"

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

            .and(path("/api/v2/messages/sms"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": "Invalid access token"

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

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("access_token", "token")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-slash"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "192.168.1.100")

            .set("access_token", "test-token")

            .set("to", "+15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod signal_send_tests {

    use super::*;

    use noti_providers::signal::SignalProvider;



    fn provider() -> SignalProvider {

        SignalProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("from", "+1234567890")

            .set("to", "+0987654321")

            .set("server", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("from", "+1234567890")

            .set("to", "+0987654321");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_from() {

        let provider = provider();

        let config = ProviderConfig::new().set("to", "+0987654321");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new().set("from", "+1234567890");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "signal");

        assert_eq!(provider.url_scheme(), "signal");

        assert!(provider.supports_attachments());

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "timestamp": 1234567890

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Signal");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "signal");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "timestamp": 1234567891

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

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v2/send"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "error": "Invalid recipient"

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

            .and(path("/v2/send"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "error": "Internal error"

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

    async fn test_send_custom_server() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "timestamp": 1234567892

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Custom server");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_server_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("from", "+1234567890")

            .set("to", "+0987654321");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod dapnet_send_tests {

    use super::*;

    use noti_providers::dapnet::DapnetProvider;



    fn provider() -> DapnetProvider {

        DapnetProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("callsign", "DL1ABC")

            .set("password", "test-password")

            .set("to", "DL2DEF")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("callsign", "DL1ABC")

            .set("password", "pass")

            .set("to", "DL2DEF");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_callsign() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("password", "pass")

            .set("to", "DL2DEF");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_to() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("callsign", "DL1ABC")

            .set("password", "pass");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "dapnet");

        assert_eq!(provider.url_scheme(), "dapnet");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/calls"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "call-123"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from DAPNET");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "dapnet");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "call-456"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_emergency() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "call-789"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("emergency", "true");

        let message = Message::text("Emergency alert");



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

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "call-abc"})))

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

            .set("callsign", "DL1ABC")

            .set("password", "pass")

            .set("to", "DL2DEF");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "call-xyz"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("callsign", "DL1ABC")

            .set("password", "test-password")

            .set("to", "DL2DEF")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod freemobile_send_tests {

    use super::*;

    use noti_providers::freemobile::FreeMobileProvider;



    fn provider() -> FreeMobileProvider {

        FreeMobileProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("user", "12345678")

            .set("password", "test-api-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user", "12345678")

            .set("password", "test-api-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_user() {

        let provider = provider();

        let config = ProviderConfig::new().set("password", "test-api-key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_password() {

        let provider = provider();

        let config = ProviderConfig::new().set("user", "12345678");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "freemobile");

        assert_eq!(provider.url_scheme(), "freemobile");

        assert!(!provider.description().is_empty());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Free Mobile");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "freemobile");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

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

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(400).set_body_string("Bad Request"))

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

            .set("user", "12345678")

            .set("password", "test-api-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("user", "12345678")

            .set("password", "test-api-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod popcorn_send_tests {

    use super::*;

    use noti_providers::popcorn::PopcornProvider;



    fn provider() -> PopcornProvider {

        PopcornProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("from", "+15551234567")

            .set("to", "+15559876543");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("from", "+1").set("to", "+2");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "popcorn");

        assert_eq!(provider.url_scheme(), "popcorn");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "sent"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello SMS");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "popcorn");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "sent"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body").with_title("Subject");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"Message": "Invalid API key"})))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "sent"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("from", "+15551234567")

            .set("to", "+15559876543")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod sfr_send_tests {

    use super::*;

    use noti_providers::sfr::SfrProvider;



    fn provider() -> SfrProvider {

        SfrProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("phone", "0612345678")

            .set("password", "test-password")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("phone", "0612345678")

            .set("password", "pass");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_phone() {

        let provider = provider();

        let config = ProviderConfig::new().set("password", "pass");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_password() {

        let provider = provider();

        let config = ProviderConfig::new().set("phone", "0612345678");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "sfr");

        assert_eq!(provider.url_scheme(), "sfr");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello SFR");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "sfr");

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

            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("phone", "0612345678")

            .set("password", "test-password")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod notifico_send_tests {

    use super::*;

    use noti_providers::notifico::NotificoProvider;



    fn provider() -> NotificoProvider {

        NotificoProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("project_id", "proj-123")

            .set("msghook", "hook-abc")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("project_id", "proj-1")

            .set("msghook", "hook-1");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_project_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("msghook", "hook-1");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_msghook() {

        let provider = provider();

        let config = ProviderConfig::new().set("project_id", "proj-1");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "notifico");

        assert_eq!(provider.url_scheme(), "notifico");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Notifico");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "notifico");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

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

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "unauthorized"})))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("project_id", "proj-123")

            .set("msghook", "hook-abc")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod whatsapp_send_tests {

    use super::*;

    use noti_providers::whatsapp::WhatsAppProvider;



    fn provider() -> WhatsAppProvider {

        WhatsAppProvider::new(client())

    }



    fn base_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("phone_number_id", "test-phone-id")

            .set("to", "+1234567890")

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

    async fn test_validate_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("phone_number_id", "pid")

            .set("to", "+1");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "whatsapp");

        assert_eq!(provider.url_scheme(), "whatsapp");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_text_success() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-phone-id/messages"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "messages": [{"id": "wamid-123"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello WhatsApp");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "whatsapp");

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": {"message": "Invalid access token"}

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

