use async_trait::async_trait;
use noti_core::{
    Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, ProviderRegistry, SendResponse,
};
use rstest::rstest;
use std::sync::Arc;

/// A minimal mock provider for testing the registry.
struct MockProvider {
    pname: &'static str,
    pscheme: &'static str,
}

impl MockProvider {
    fn new(name: &'static str, scheme: &'static str) -> Self {
        Self {
            pname: name,
            pscheme: scheme,
        }
    }
}

#[async_trait]
impl NotifyProvider for MockProvider {
    fn name(&self) -> &str {
        self.pname
    }
    fn url_scheme(&self) -> &str {
        self.pscheme
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::required("token", "A token")]
    }
    fn description(&self) -> &str {
        "mock provider"
    }
    fn example_url(&self) -> &str {
        "mock://token"
    }
    async fn send(
        &self,
        _message: &Message,
        _config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        Ok(SendResponse::success(self.pname, "sent"))
    }
}

#[rstest]
fn test_registry_new_is_empty() {
    let registry = ProviderRegistry::new();
    assert!(registry.all_providers().is_empty());
    assert!(registry.provider_names().is_empty());
}

#[rstest]
fn test_registry_default_is_empty() {
    let registry = ProviderRegistry::default();
    assert!(registry.all_providers().is_empty());
}

#[rstest]
fn test_registry_register_and_get_by_name() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new("test", "test")));

    let provider = registry.get_by_name("test");
    assert!(provider.is_some());
    assert_eq!(provider.unwrap().name(), "test");
}

#[rstest]
fn test_registry_get_by_name_not_found() {
    let registry = ProviderRegistry::new();
    assert!(registry.get_by_name("nonexistent").is_none());
}

#[rstest]
fn test_registry_get_by_scheme() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new("telegram", "tg")));

    let provider = registry.get_by_scheme("tg");
    assert!(provider.is_some());
    assert_eq!(provider.unwrap().name(), "telegram");
}

#[rstest]
fn test_registry_get_by_scheme_not_found() {
    let registry = ProviderRegistry::new();
    assert!(registry.get_by_scheme("nonexistent").is_none());
}

#[rstest]
fn test_registry_all_providers() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new("alpha", "alpha")));
    registry.register(Arc::new(MockProvider::new("beta", "beta")));
    registry.register(Arc::new(MockProvider::new("gamma", "gamma")));

    let all = registry.all_providers();
    assert_eq!(all.len(), 3);
}

#[rstest]
fn test_registry_provider_names_sorted() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new("zulu", "zulu")));
    registry.register(Arc::new(MockProvider::new("alpha", "alpha")));
    registry.register(Arc::new(MockProvider::new("mike", "mike")));

    let names = registry.provider_names();
    assert_eq!(names, vec!["alpha", "mike", "zulu"]);
}

#[rstest]
fn test_registry_register_overwrites_same_name() {
    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(MockProvider::new("test", "scheme1")));
    registry.register(Arc::new(MockProvider::new("test", "scheme2")));

    // Should have only 1 provider with name "test"
    let provider = registry.get_by_name("test").unwrap();
    assert_eq!(provider.url_scheme(), "scheme2");
    // But both schemes map
    assert!(registry.get_by_scheme("scheme2").is_some());
}

// ======================== NotifyProvider trait default validate_config ========================

#[rstest]
fn test_validate_config_default_impl_success() {
    let provider = MockProvider::new("test", "test");
    let config = ProviderConfig::new().set("token", "abc123");
    assert!(provider.validate_config(&config).is_ok());
}

#[rstest]
fn test_validate_config_default_impl_missing_required() {
    let provider = MockProvider::new("test", "test");
    let config = ProviderConfig::new();
    let result = provider.validate_config(&config);
    assert!(result.is_err());
    match result.unwrap_err() {
        NotiError::Validation(msg) => {
            assert!(msg.contains("token"));
            assert!(msg.contains("test"));
        }
        _ => panic!("expected Validation error"),
    }
}

/// Provider with optional params only — should always pass validate.
struct OptionalOnlyProvider;

#[async_trait]
impl NotifyProvider for OptionalOnlyProvider {
    fn name(&self) -> &str {
        "optional_only"
    }
    fn url_scheme(&self) -> &str {
        "opt"
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::optional("channel", "optional channel")]
    }
    fn description(&self) -> &str {
        "test"
    }
    fn example_url(&self) -> &str {
        "opt://..."
    }
    async fn send(
        &self,
        _message: &Message,
        _config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        Ok(SendResponse::success("optional_only", "sent"))
    }
}

#[rstest]
fn test_validate_config_optional_params_always_pass() {
    let provider = OptionalOnlyProvider;
    let config = ProviderConfig::new(); // empty
    assert!(provider.validate_config(&config).is_ok());
}

/// Provider with no params at all — should always pass.
struct NoParamProvider;

#[async_trait]
impl NotifyProvider for NoParamProvider {
    fn name(&self) -> &str {
        "no_param"
    }
    fn url_scheme(&self) -> &str {
        "nop"
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![]
    }
    fn description(&self) -> &str {
        "test"
    }
    fn example_url(&self) -> &str {
        "nop://..."
    }
    async fn send(
        &self,
        _message: &Message,
        _config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        Ok(SendResponse::success("no_param", "sent"))
    }
}

#[rstest]
fn test_validate_config_no_params_always_pass() {
    let provider = NoParamProvider;
    let config = ProviderConfig::new();
    assert!(provider.validate_config(&config).is_ok());
}
