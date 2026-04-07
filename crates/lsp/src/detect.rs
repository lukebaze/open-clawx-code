//! Auto-detect project languages from marker files and map to LSP server commands.

use std::path::Path;

/// Configuration for a detected language and its LSP server.
#[derive(Debug, Clone)]
pub struct LanguageConfig {
    pub language: String,
    pub server_command: String,
    pub server_args: Vec<String>,
    pub root_markers: Vec<String>,
}

/// Scan project root for language marker files and return matching configs.
#[must_use]
pub fn detect_languages(root: &Path) -> Vec<LanguageConfig> {
    let mut configs = Vec::new();

    // Rust: Cargo.toml
    if root.join("Cargo.toml").exists() {
        configs.push(LanguageConfig {
            language: "rust".to_string(),
            server_command: "rust-analyzer".to_string(),
            server_args: vec![],
            root_markers: vec!["Cargo.toml".to_string()],
        });
    }

    // Python: pyproject.toml, requirements.txt, setup.py
    if root.join("pyproject.toml").exists()
        || root.join("requirements.txt").exists()
        || root.join("setup.py").exists()
    {
        configs.push(LanguageConfig {
            language: "python".to_string(),
            server_command: "pyright-langserver".to_string(),
            server_args: vec!["--stdio".to_string()],
            root_markers: vec![
                "pyproject.toml".to_string(),
                "requirements.txt".to_string(),
            ],
        });
    }

    // TypeScript/JavaScript: package.json, tsconfig.json
    if root.join("package.json").exists() || root.join("tsconfig.json").exists() {
        configs.push(LanguageConfig {
            language: "typescript".to_string(),
            server_command: "typescript-language-server".to_string(),
            server_args: vec!["--stdio".to_string()],
            root_markers: vec![
                "package.json".to_string(),
                "tsconfig.json".to_string(),
            ],
        });
    }

    // Go: go.mod
    if root.join("go.mod").exists() {
        configs.push(LanguageConfig {
            language: "go".to_string(),
            server_command: "gopls".to_string(),
            server_args: vec![],
            root_markers: vec!["go.mod".to_string()],
        });
    }

    configs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    #[test]
    fn detects_rust_project() {
        let dir = temp_dir();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let configs = detect_languages(dir.path());
        assert!(configs.iter().any(|c| c.language == "rust"));
    }

    #[test]
    fn detects_python_via_requirements() {
        let dir = temp_dir();
        fs::write(dir.path().join("requirements.txt"), "").unwrap();
        let configs = detect_languages(dir.path());
        assert!(configs.iter().any(|c| c.language == "python"));
    }

    #[test]
    fn detects_typescript_via_package_json() {
        let dir = temp_dir();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        let configs = detect_languages(dir.path());
        assert!(configs.iter().any(|c| c.language == "typescript"));
    }

    #[test]
    fn detects_go_via_go_mod() {
        let dir = temp_dir();
        fs::write(dir.path().join("go.mod"), "module example").unwrap();
        let configs = detect_languages(dir.path());
        assert!(configs.iter().any(|c| c.language == "go"));
    }

    #[test]
    fn empty_dir_yields_no_configs() {
        let dir = temp_dir();
        let configs = detect_languages(dir.path());
        assert!(configs.is_empty());
    }

    #[test]
    fn multi_language_project_detected() {
        let dir = temp_dir();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        let configs = detect_languages(dir.path());
        assert_eq!(configs.len(), 2);
    }
}
