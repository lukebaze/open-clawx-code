use std::collections::HashMap;

use crate::{ModelInfo, Provider};

/// Registry managing multiple providers and the currently active model.
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
    active_model: String,
}

impl ProviderRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active_model: String::new(),
        }
    }

    /// Create a registry from auto-detected providers.
    #[must_use]
    pub fn from_detected(providers: Vec<Box<dyn Provider>>) -> Self {
        let mut registry = Self::new();
        for provider in providers {
            registry.register(provider);
        }
        // Default to first available model
        if let Some((_, model)) = registry.all_models().first() {
            registry.active_model.clone_from(&model.id);
        }
        registry
    }

    /// Register a provider.
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    /// Set the active model by ID (e.g., "claude-sonnet-4-20250514").
    pub fn set_active_model(&mut self, model_id: &str) -> anyhow::Result<()> {
        // Verify the model exists in some provider
        let exists = self
            .providers
            .values()
            .any(|p| p.models().iter().any(|m| m.id == model_id));
        if exists {
            self.active_model = model_id.to_string();
            Ok(())
        } else {
            anyhow::bail!("Model '{model_id}' not found in any provider")
        }
    }

    /// Get the currently active model ID.
    #[must_use]
    pub fn active_model(&self) -> &str {
        &self.active_model
    }

    /// Get the provider for the currently active model.
    #[must_use]
    pub fn active_provider(&self) -> Option<&dyn Provider> {
        for provider in self.providers.values() {
            if provider.models().iter().any(|m| m.id == self.active_model) {
                return Some(provider.as_ref());
            }
        }
        None
    }

    /// List all models from all providers.
    #[must_use]
    pub fn all_models(&self) -> Vec<(String, ModelInfo)> {
        let mut models = Vec::new();
        for provider in self.providers.values() {
            for model in provider.models() {
                models.push((provider.name().to_string(), model));
            }
        }
        models
    }

    /// Number of registered providers.
    #[must_use]
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
