use std::collections::HashMap;
use std::sync::Arc;

use crate::provider::NotifyProvider;

/// Registry that maps provider names and URL schemes to provider instances.
pub struct ProviderRegistry {
    /// Map from provider name to provider instance.
    by_name: HashMap<String, Arc<dyn NotifyProvider>>,
    /// Map from URL scheme to provider name.
    scheme_to_name: HashMap<String, String>,
}

impl ProviderRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            scheme_to_name: HashMap::new(),
        }
    }

    /// Register a provider instance.
    pub fn register(&mut self, provider: Arc<dyn NotifyProvider>) {
        let name = provider.name().to_string();
        let scheme = provider.url_scheme().to_string();
        self.scheme_to_name.insert(scheme, name.clone());
        self.by_name.insert(name, provider);
    }

    /// Look up a provider by name.
    pub fn get_by_name(&self, name: &str) -> Option<&Arc<dyn NotifyProvider>> {
        self.by_name.get(name)
    }

    /// Look up a provider by URL scheme.
    pub fn get_by_scheme(&self, scheme: &str) -> Option<&Arc<dyn NotifyProvider>> {
        let name = self.scheme_to_name.get(scheme)?;
        self.by_name.get(name)
    }

    /// Get all registered providers.
    pub fn all_providers(&self) -> Vec<&Arc<dyn NotifyProvider>> {
        self.by_name.values().collect()
    }

    /// Get all registered provider names, sorted alphabetically.
    pub fn provider_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.by_name.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
