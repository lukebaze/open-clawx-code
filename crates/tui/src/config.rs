//! User configuration — persists API keys and preferences to disk.
//!
//! Stored at `~/.open-clawx-code/config.toml`. Keys are loaded into
//! environment variables on startup so providers auto-detect them.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// A custom OpenAI-compatible provider endpoint.
#[derive(Debug, Clone)]
pub struct CustomProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
}

/// User configuration loaded from `~/.open-clawx-code/config.toml`
#[derive(Debug, Clone, Default)]
pub struct UserConfig {
    /// Provider API keys: `provider_name` → key value.
    pub api_keys: BTreeMap<String, String>,
    /// Default model ID.
    pub default_model: Option<String>,
    /// Custom OpenAI-compatible providers.
    pub custom_providers: Vec<CustomProvider>,
}

impl UserConfig {
    /// Config directory: `~/.open-clawx-code/`
    #[must_use]
    pub fn config_dir() -> PathBuf {
        dirs_fallback().join(".open-clawx-code")
    }

    /// Full path to config file.
    #[must_use]
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Load config from disk. Returns default if file doesn't exist.
    #[must_use]
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        Self::parse(&content)
    }

    /// Save config to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let content = self.serialize();
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }

    /// Apply API keys to environment variables so providers detect them.
    pub fn apply_to_env(&self) {
        for (provider, key) in &self.api_keys {
            let env_var = provider_to_env_var(provider);
            if !key.is_empty() {
                std::env::set_var(&env_var, key);
            }
        }
    }

    /// Set an API key for a provider.
    pub fn set_key(&mut self, provider: &str, key: String) {
        self.api_keys.insert(provider.to_string(), key);
    }

    /// Get an API key for a provider.
    #[must_use]
    pub fn get_key(&self, provider: &str) -> Option<&str> {
        self.api_keys.get(provider).map(String::as_str)
    }

    /// Known provider names for the config editor.
    #[must_use]
    pub fn known_providers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("anthropic", "ANTHROPIC_API_KEY"),
            ("openai", "OPENAI_API_KEY"),
            ("gemini", "GEMINI_API_KEY"),
            ("groq", "GROQ_API_KEY"),
            ("ollama", "(no key needed)"),
        ]
    }

    /// Add a custom provider.
    pub fn add_custom_provider(&mut self, provider: CustomProvider) {
        // Remove existing with same name
        self.custom_providers.retain(|p| p.name != provider.name);
        self.custom_providers.push(provider);
    }

    /// Remove a custom provider by name.
    pub fn remove_custom_provider(&mut self, name: &str) {
        self.custom_providers.retain(|p| p.name != name);
    }

    fn parse(content: &str) -> Self {
        let mut config = Self::default();
        let mut section = String::new();
        let mut current_custom: Option<CustomProvider> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                // Flush previous custom provider
                if let Some(cp) = current_custom.take() {
                    if !cp.name.is_empty() {
                        config.custom_providers.push(cp);
                    }
                }
                section = line
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .to_string();
                // Detect custom provider sections: [provider.myname]
                if let Some(name) = section.strip_prefix("provider.") {
                    current_custom = Some(CustomProvider {
                        name: name.to_string(),
                        base_url: String::new(),
                        api_key: String::new(),
                        models: Vec::new(),
                    });
                }
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim().trim_matches('"');
                let v = v.trim().trim_matches('"');
                if let Some(cp) = &mut current_custom {
                    match k {
                        "base_url" => cp.base_url = v.to_string(),
                        "api_key" => cp.api_key = v.to_string(),
                        "models" => {
                            cp.models = v
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                        _ => {}
                    }
                } else {
                    match section.as_str() {
                        "api_keys" => {
                            config.api_keys.insert(k.to_string(), v.to_string());
                        }
                        "general" if k == "default_model" => {
                            config.default_model = Some(v.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
        // Flush last custom provider
        if let Some(cp) = current_custom {
            if !cp.name.is_empty() {
                config.custom_providers.push(cp);
            }
        }
        config
    }

    fn serialize(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        out.push_str("[general]\n");
        if let Some(model) = &self.default_model {
            let _ = writeln!(out, "default_model = \"{model}\"");
        }
        out.push('\n');
        out.push_str("[api_keys]\n");
        for (provider, key) in &self.api_keys {
            let _ = writeln!(out, "{provider} = \"{key}\"");
        }
        for cp in &self.custom_providers {
            let _ = writeln!(out, "\n[provider.{}]", cp.name);
            let _ = writeln!(out, "base_url = \"{}\"", cp.base_url);
            let _ = writeln!(out, "api_key = \"{}\"", cp.api_key);
            let _ = writeln!(out, "models = \"{}\"", cp.models.join(", "));
        }
        out
    }
}

/// Map provider name to environment variable name.
fn provider_to_env_var(provider: &str) -> String {
    match provider {
        "anthropic" => "ANTHROPIC_API_KEY".to_string(),
        "openai" => "OPENAI_API_KEY".to_string(),
        "gemini" => "GEMINI_API_KEY".to_string(),
        "groq" => "GROQ_API_KEY".to_string(),
        _ => format!("{}_API_KEY", provider.to_uppercase()),
    }
}

/// Get home directory with fallback.
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_serialize_roundtrip() {
        let toml = r#"
[general]
default_model = "claude-sonnet-4-20250514"

[api_keys]
anthropic = "sk-ant-test123"
openai = "sk-test456"
"#;
        let config = UserConfig::parse(toml);
        assert_eq!(
            config.default_model.as_deref(),
            Some("claude-sonnet-4-20250514")
        );
        assert_eq!(config.api_keys.get("anthropic").unwrap(), "sk-ant-test123");
        assert_eq!(config.api_keys.get("openai").unwrap(), "sk-test456");

        let serialized = config.serialize();
        let reparsed = UserConfig::parse(&serialized);
        assert_eq!(reparsed.default_model, config.default_model);
        assert_eq!(reparsed.api_keys, config.api_keys);
    }

    #[test]
    fn empty_config_returns_default() {
        let config = UserConfig::parse("");
        assert!(config.api_keys.is_empty());
        assert!(config.default_model.is_none());
    }

    #[test]
    fn provider_env_vars_mapped_correctly() {
        assert_eq!(provider_to_env_var("anthropic"), "ANTHROPIC_API_KEY");
        assert_eq!(provider_to_env_var("openai"), "OPENAI_API_KEY");
        assert_eq!(provider_to_env_var("custom"), "CUSTOM_API_KEY");
    }

    #[test]
    fn custom_provider_roundtrip() {
        let toml = r#"
[general]
default_model = "my-model"

[api_keys]
anthropic = "sk-test"

[provider.deepseek]
base_url = "https://api.deepseek.com/v1"
api_key = "sk-deep-123"
models = "deepseek-chat, deepseek-coder"

[provider.local-llama]
base_url = "http://localhost:11434/v1"
api_key = ""
models = "llama3"
"#;
        let config = UserConfig::parse(toml);
        assert_eq!(config.custom_providers.len(), 2);
        assert_eq!(config.custom_providers[0].name, "deepseek");
        assert_eq!(
            config.custom_providers[0].base_url,
            "https://api.deepseek.com/v1"
        );
        assert_eq!(config.custom_providers[0].models.len(), 2);
        assert_eq!(config.custom_providers[1].name, "local-llama");

        // Roundtrip
        let serialized = config.serialize();
        let reparsed = UserConfig::parse(&serialized);
        assert_eq!(reparsed.custom_providers.len(), 2);
        assert_eq!(reparsed.custom_providers[0].name, "deepseek");
        assert_eq!(reparsed.custom_providers[0].models, vec!["deepseek-chat", "deepseek-coder"]);
    }
}
