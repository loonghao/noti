/// Extended wiremock-based send() tests for providers with configurable URLs.
/// Each test covers the send() success+failure paths to maximize line coverage.
use noti_core::{Message, MessageFormat, NotifyProvider, ProviderConfig};
use reqwest::Client;
use url::Url;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client() -> Client {
    Client::new()
}

fn parse_mock(uri: &str) -> (String, String) {
    let u = Url::parse(uri).unwrap();
    (
        u.host_str().unwrap().to_string(),
        u.port().unwrap().to_string(),
    )
}

// ==================== form_webhook ====================
mod form_webhook_tests {
    use super::*;
    use noti_providers::form_webhook::FormWebhookProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = FormWebhookProvider::new(client());
        let c = ProviderConfig::new().set("url", ms.uri());
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "form");
    }

    #[tokio::test]
    async fn test_send_with_title_and_type() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = FormWebhookProvider::new(client());
        let c = ProviderConfig::new()
            .set("url", ms.uri())
            .set("type", "warning");
        let r = p
            .send(&Message::text("body").with_title("Title"), &c)
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_with_headers() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("X-Api-Key", "abc"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = FormWebhookProvider::new(client());
        let c = ProviderConfig::new()
            .set("url", ms.uri())
            .set("header", "X-Api-Key=abc");
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("err"))
            .mount(&ms)
            .await;
        let p = FormWebhookProvider::new(client());
        let c = ProviderConfig::new().set("url", ms.uri());
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(!r.success);
        assert_eq!(r.status_code, Some(500));
    }
}

// ==================== json_webhook ====================
mod json_webhook_tests {
    use super::*;
    use noti_providers::json_webhook::JsonWebhookProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&ms)
            .await;
        let p = JsonWebhookProvider::new(client());
        let c = ProviderConfig::new().set("url", ms.uri());
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "json");
    }

    #[tokio::test]
    async fn test_send_with_title_and_headers() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = JsonWebhookProvider::new(client());
        let c = ProviderConfig::new()
            .set("url", ms.uri())
            .set("header", "X-Key=val")
            .set("type", "error");
        let r = p
            .send(&Message::text("body").with_title("T"), &c)
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_string("denied"))
            .mount(&ms)
            .await;
        let p = JsonWebhookProvider::new(client());
        let c = ProviderConfig::new().set("url", ms.uri());
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== xml_webhook ====================
mod xml_webhook_tests {
    use super::*;
    use noti_providers::xml_webhook::XmlWebhookProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<ok/>"))
            .mount(&ms)
            .await;
        let p = XmlWebhookProvider::new(client());
        let c = ProviderConfig::new().set("url", ms.uri());
        let r = p.send(&Message::text("hello & world"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "xml");
    }

    #[tokio::test]
    async fn test_send_with_title_and_custom_root() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = XmlWebhookProvider::new(client());
        let c = ProviderConfig::new()
            .set("url", ms.uri())
            .set("root", "alert")
            .set("header", "X-H=v");
        let r = p
            .send(&Message::text("body").with_title("Title <&>"), &c)
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
            .mount(&ms)
            .await;
        let p = XmlWebhookProvider::new(client());
        let c = ProviderConfig::new().set("url", ms.uri());
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== teams ====================
mod teams_tests {
    use super::*;
    use noti_providers::teams::TeamsProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("1"))
            .mount(&ms)
            .await;
        let p = TeamsProvider::new(client());
        let c = ProviderConfig::new().set("webhook_url", ms.uri());
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "teams");
    }

    #[tokio::test]
    async fn test_send_with_title_markdown() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("1"))
            .mount(&ms)
            .await;
        let p = TeamsProvider::new(client());
        let c = ProviderConfig::new()
            .set("webhook_url", ms.uri())
            .set("theme_color", "FF0000");
        let r = p
            .send(
                &Message::text("**bold**")
                    .with_title("Alert")
                    .with_format(MessageFormat::Markdown),
                &c,
            )
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_string("denied"))
            .mount(&ms)
            .await;
        let p = TeamsProvider::new(client());
        let c = ProviderConfig::new().set("webhook_url", ms.uri());
        let r = p.send(&Message::text("hi"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== spike ====================
mod spike_tests {
    use super::*;
    use noti_providers::spike::SpikeProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = SpikeProvider::new(client());
        let c = ProviderConfig::new().set("webhook_url", ms.uri());
        let r = p
            .send(&Message::text("alert").with_title("Incident"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "spike");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauth"))
            .mount(&ms)
            .await;
        let p = SpikeProvider::new(client());
        let c = ProviderConfig::new().set("webhook_url", ms.uri());
        let r = p.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== twist ====================
mod twist_tests {
    use super::*;
    use noti_providers::twist::TwistProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = TwistProvider::new(client());
        let c = ProviderConfig::new().set("webhook_url", ms.uri());
        let r = p
            .send(&Message::text("hello").with_title("Title"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "twist");
    }

    #[tokio::test]
    async fn test_send_no_title() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let p = TwistProvider::new(client());
        let c = ProviderConfig::new().set("webhook_url", ms.uri());
        let r = p.send(&Message::text("no title"), &c).await.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("err"))
            .mount(&ms)
            .await;
        let p = TwistProvider::new(client());
        let c = ProviderConfig::new().set("webhook_url", ms.uri());
        let r = p.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== rocketchat ====================
mod rocketchat_tests {
    use super::*;
    use noti_providers::rocketchat::RocketChatProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"success":true})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = RocketChatProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("token_a", "a")
            .set("token_b", "b")
            .set("port", &p)
            .set("scheme", "http");
        let r = prov.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "rocketchat");
    }

    #[tokio::test]
    async fn test_send_with_opts_markdown() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"success":true})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = RocketChatProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("token_a", "a")
            .set("token_b", "b")
            .set("port", &p)
            .set("scheme", "http")
            .set("channel", "#general")
            .set("username", "Bot")
            .set("icon_url", "https://x.com/i.png");
        let r = prov
            .send(
                &Message::text("md text")
                    .with_title("T")
                    .with_format(MessageFormat::Markdown),
                &c,
            )
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_with_title_text() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"success":true})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = RocketChatProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("token_a", "a")
            .set("token_b", "b")
            .set("port", &p)
            .set("scheme", "http");
        let r = prov
            .send(&Message::text("body").with_title("Title"), &c)
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_json(serde_json::json!({"success":false,"error":"forbidden"})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = RocketChatProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("token_a", "a")
            .set("token_b", "b")
            .set("port", &p)
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== homeassistant ====================
mod homeassistant_tests {
    use super::*;
    use noti_providers::homeassistant::HomeAssistantProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("Authorization", "Bearer tok123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = HomeAssistantProvider::new(client());
        let c = ProviderConfig::new()
            .set("access_token", "tok123")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http");
        let r = prov
            .send(&Message::text("test").with_title("Alert"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "homeassistant");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"message":"unauthorized"})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = HomeAssistantProvider::new(client());
        let c = ProviderConfig::new()
            .set("access_token", "bad")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== emby ====================
mod emby_tests {
    use super::*;
    use noti_providers::emby::EmbyProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("X-Emby-Token", "key123"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = EmbyProvider::new(client());
        let c = ProviderConfig::new()
            .set("api_key", "key123")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http");
        let r = prov
            .send(&Message::text("test").with_title("T"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "emby");
    }

    #[tokio::test]
    async fn test_send_with_user_id() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = EmbyProvider::new(client());
        let c = ProviderConfig::new()
            .set("api_key", "key")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http")
            .set("user_id", "u123");
        let r = prov.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauth"))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = EmbyProvider::new(client());
        let c = ProviderConfig::new()
            .set("api_key", "bad")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== jellyfin ====================
mod jellyfin_tests {
    use super::*;
    use noti_providers::jellyfin::JellyfinProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(204).set_body_string(""))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = JellyfinProvider::new(client());
        let c = ProviderConfig::new()
            .set("api_key", "k")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http");
        let r = prov.send(&Message::text("test"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "jellyfin");
    }

    #[tokio::test]
    async fn test_send_with_user_id() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = JellyfinProvider::new(client());
        let c = ProviderConfig::new()
            .set("api_key", "k")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http")
            .set("user_id", "uid");
        let r = prov
            .send(&Message::text("t").with_title("T"), &c)
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("err"))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = JellyfinProvider::new(client());
        let c = ProviderConfig::new()
            .set("api_key", "k")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== chanify ====================
mod chanify_tests {
    use super::*;
    use noti_providers::chanify::ChanifyProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let prov = ChanifyProvider::new(client());
        let c = ProviderConfig::new()
            .set("token", "tok")
            .set("server", ms.uri());
        let r = prov
            .send(&Message::text("hi").with_title("T"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "chanify");
    }

    #[tokio::test]
    async fn test_send_no_title() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let prov = ChanifyProvider::new(client());
        let c = ProviderConfig::new()
            .set("token", "tok")
            .set("server", ms.uri());
        let r = prov.send(&Message::text("no title"), &c).await.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
            .mount(&ms)
            .await;
        let prov = ChanifyProvider::new(client());
        let c = ProviderConfig::new()
            .set("token", "tok")
            .set("server", ms.uri());
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== pushdeer ====================
mod pushdeer_tests {
    use super::*;
    use noti_providers::pushdeer::PushDeerProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"code":0,"content":{}})),
            )
            .mount(&ms)
            .await;
        let prov = PushDeerProvider::new(client());
        let c = ProviderConfig::new()
            .set("push_key", "PDU123")
            .set("server", ms.uri());
        let r = prov
            .send(
                &Message::text("hi")
                    .with_title("T")
                    .with_format(MessageFormat::Markdown),
                &c,
            )
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "pushdeer");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"code":-1,"error":"invalid key"})),
            )
            .mount(&ms)
            .await;
        let prov = PushDeerProvider::new(client());
        let c = ProviderConfig::new()
            .set("push_key", "bad")
            .set("server", ms.uri());
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== pushjet ====================
mod pushjet_tests {
    use super::*;
    use noti_providers::pushjet::PushjetProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let prov = PushjetProvider::new(client());
        let c = ProviderConfig::new()
            .set("secret", "sec")
            .set("server", ms.uri())
            .set("level", "5")
            .set("link", "https://x.com");
        let r = prov
            .send(&Message::text("hi").with_title("T"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "pushjet");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauth"))
            .mount(&ms)
            .await;
        let prov = PushjetProvider::new(client());
        let c = ProviderConfig::new()
            .set("secret", "bad")
            .set("server", ms.uri());
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== signal ====================
mod signal_tests {
    use super::*;
    use noti_providers::signal::SignalProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({})))
            .mount(&ms)
            .await;
        let prov = SignalProvider::new(client());
        let c = ProviderConfig::new()
            .set("from", "+1234")
            .set("to", "+5678")
            .set("server", ms.uri());
        let r = prov
            .send(&Message::text("hi").with_title("Urgent"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "signal");
    }

    #[tokio::test]
    async fn test_send_no_title() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&ms)
            .await;
        let prov = SignalProvider::new(client());
        let c = ProviderConfig::new()
            .set("from", "+1")
            .set("to", "+2")
            .set("server", ms.uri());
        let r = prov.send(&Message::text("plain"), &c).await.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"error":"invalid number"})),
            )
            .mount(&ms)
            .await;
        let prov = SignalProvider::new(client());
        let c = ProviderConfig::new()
            .set("from", "+1")
            .set("to", "+2")
            .set("server", ms.uri());
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== jira ====================
mod jira_tests {
    use super::*;
    use noti_providers::jira::JiraProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"id":"123"})))
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = JiraProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", format!("{h}:{p}"))
            .set("user", "u@e.com")
            .set("api_token", "tok")
            .set("issue_key", "PROJ-1")
            .set("scheme", "http");
        let r = prov.send(&Message::text("comment"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "jira");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({"errorMessages":["Issue Does Not Exist"]})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = JiraProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", format!("{h}:{p}"))
            .set("user", "u")
            .set("api_token", "t")
            .set("issue_key", "X-0")
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== nextcloud ====================
mod nextcloud_tests {
    use super::*;
    use noti_providers::nextcloud::NextcloudProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"ocs":{"meta":{"status":"ok"}}})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = NextcloudProvider::new(client());
        let c = ProviderConfig::new()
            .set("user", "admin")
            .set("password", "pass")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http")
            .set("target_user", "john");
        let r = prov
            .send(&Message::text("hi").with_title("Alert"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "nextcloud");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"ocs":{"meta":{"message":"unauthorized"}}})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = NextcloudProvider::new(client());
        let c = ProviderConfig::new()
            .set("user", "u")
            .set("password", "p")
            .set("host", format!("{h}:{p}"))
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== nctalk ====================
mod nctalk_tests {
    use super::*;
    use noti_providers::nctalk::NcTalkProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(serde_json::json!({"ocs":{"data":{}}})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = NcTalkProvider::new(client());
        let c = ProviderConfig::new()
            .set("user", "u")
            .set("password", "p")
            .set("host", format!("{h}:{p}"))
            .set("room_token", "r1")
            .set("scheme", "http");
        let r = prov.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "nctalk");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(
                    serde_json::json!({"ocs":{"meta":{"message":"room not found"}}}),
                ),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = NcTalkProvider::new(client());
        let c = ProviderConfig::new()
            .set("user", "u")
            .set("password", "p")
            .set("host", format!("{h}:{p}"))
            .set("room_token", "bad")
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== apprise ====================
mod apprise_tests {
    use super::*;
    use noti_providers::apprise::AppriseProvider;

    #[tokio::test]
    async fn test_send_success_with_config_key() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let prov = AppriseProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", ms.uri())
            .set("config_key", "my-config")
            .set("tag", "all");
        let r = prov
            .send(
                &Message::text("hi")
                    .with_title("T")
                    .with_format(MessageFormat::Markdown),
                &c,
            )
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "apprise");
    }

    #[tokio::test]
    async fn test_send_success_with_urls() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&ms)
            .await;
        let prov = AppriseProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", ms.uri())
            .set("urls", "slack://tok");
        let r = prov
            .send(&Message::text("hi").with_format(MessageFormat::Html), &c)
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(424).set_body_string("failed"))
            .mount(&ms)
            .await;
        let prov = AppriseProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", ms.uri())
            .set("config_key", "k");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== kodi ====================
mod kodi_tests {
    use super::*;
    use noti_providers::kodi::KodiProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id":1,"jsonrpc":"2.0","result":"OK"})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = KodiProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("port", &p)
            .set("scheme", "http")
            .set("user", "kodi")
            .set("password", "kodi");
        let r = prov
            .send(&Message::text("Playing").with_title("Now"), &c)
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "kodi");
    }

    #[tokio::test]
    async fn test_send_no_auth() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":"OK"})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = KodiProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("port", &p)
            .set("scheme", "http")
            .set("display_time", "3000")
            .set("image", "warning");
        let r = prov.send(&Message::text("t"), &c).await.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(403).set_body_json(serde_json::json!({"error":"forbidden"})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = KodiProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("port", &p)
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}

// ==================== synology ====================
mod synology_tests {
    use super::*;
    use noti_providers::synology::SynologyProvider;

    #[tokio::test]
    async fn test_send_success() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"success":true})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = SynologyProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("token", "tok")
            .set("port", &p)
            .set("scheme", "http");
        let r = prov.send(&Message::text("hi"), &c).await.unwrap();
        assert!(r.success);
        assert_eq!(r.provider, "synology");
    }

    #[tokio::test]
    async fn test_send_failure() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(403).set_body_json(serde_json::json!({"success":false})),
            )
            .mount(&ms)
            .await;
        let (h, p) = parse_mock(&ms.uri());
        let prov = SynologyProvider::new(client());
        let c = ProviderConfig::new()
            .set("host", &h)
            .set("token", "bad")
            .set("port", &p)
            .set("scheme", "http");
        let r = prov.send(&Message::text("x"), &c).await.unwrap();
        assert!(!r.success);
    }
}
