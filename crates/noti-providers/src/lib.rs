pub mod discord;
pub mod email;
pub mod feishu;
pub mod slack;
pub mod telegram;
pub mod webhook;
pub mod wecom;

use noti_core::ProviderRegistry;
use reqwest::Client;
use std::sync::Arc;

/// Register all built-in notification providers into the given registry.
pub fn register_all_providers(registry: &mut ProviderRegistry) {
    let client = Client::new();

    registry.register(Arc::new(wecom::WeComProvider::new(client.clone())));
    registry.register(Arc::new(feishu::FeishuProvider::new(client.clone())));
    registry.register(Arc::new(slack::SlackProvider::new(client.clone())));
    registry.register(Arc::new(telegram::TelegramProvider::new(client.clone())));
    registry.register(Arc::new(discord::DiscordProvider::new(client.clone())));
    registry.register(Arc::new(email::EmailProvider::new()));
    registry.register(Arc::new(webhook::WebhookProvider::new(client)));
}
