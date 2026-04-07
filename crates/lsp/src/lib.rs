//! LSP client manager — connects to language servers for code intelligence.
//!
//! Manages per-language server connections (stdio transport), providing
//! hover, goto-definition, diagnostics, and did-change notifications.

pub mod connection;
pub mod detect;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use connection::LspConnection;
pub use detect::{detect_languages, LanguageConfig};

/// Diagnostic severity levels (matching LSP spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warning => "WARN",
            Self::Information => "INFO",
            Self::Hint => "HINT",
        }
    }
}

/// A diagnostic entry from a language server.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: String,
}

/// Hover information returned by a language server.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
    pub language: String,
}

/// A source location (file + line/col position).
#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub col: u32,
}

/// Manages connections to multiple language servers.
pub struct LspManager {
    root: PathBuf,
    connections: HashMap<String, LspConnection>,
    diagnostics: Vec<Diagnostic>,
}

impl LspManager {
    /// Create a new LSP manager for a project root.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            connections: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Auto-detect languages in the project root and start appropriate servers.
    pub fn start_for_project(&mut self) {
        let configs = detect_languages(&self.root);
        for config in configs {
            match LspConnection::start(&config, &self.root) {
                Ok(conn) => {
                    self.connections.insert(config.language.clone(), conn);
                }
                Err(e) => {
                    eprintln!("LSP: failed to start {} server: {e}", config.language);
                }
            }
        }
    }

    /// Get hover info at a file position. Returns None if no server handles the file.
    #[must_use]
    pub fn hover(&self, file: &str, line: u32, col: u32) -> Option<HoverInfo> {
        let lang = self.language_for_file(file)?;
        let conn = self.connections.get(lang)?;
        conn.hover(file, line, col)
    }

    /// Get go-to-definition location. Returns None if no server handles the file.
    #[must_use]
    pub fn goto_definition(&self, file: &str, line: u32, col: u32) -> Option<Location> {
        let lang = self.language_for_file(file)?;
        let conn = self.connections.get(lang)?;
        conn.goto_definition(file, line, col)
    }

    /// All current diagnostics across all files.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Notify the appropriate language server of a file content change.
    pub fn did_change(&mut self, file: &str, content: &str) {
        if let Some(lang) = self.language_for_file(file) {
            if let Some(conn) = self.connections.get_mut(lang) {
                conn.did_change(file, content);
            }
        }
    }

    /// Replace diagnostics for the affected file(s) with new entries.
    pub fn push_diagnostics(&mut self, new_diags: Vec<Diagnostic>) {
        if let Some(first) = new_diags.first() {
            let file = first.file.clone();
            self.diagnostics.retain(|d| d.file != file);
        }
        self.diagnostics.extend(new_diags);
    }

    /// Number of active language server connections.
    #[must_use]
    pub fn active_connections(&self) -> usize {
        self.connections.len()
    }

    /// Gracefully shut down all language servers and clear state.
    pub fn stop_all(&mut self) {
        for (_, mut conn) in self.connections.drain() {
            conn.shutdown();
        }
        self.diagnostics.clear();
    }

    /// Map a file path to a language key based on extension.
    #[allow(clippy::unused_self)]
    fn language_for_file(&self, file: &str) -> Option<&'static str> {
        let ext = Path::new(file).extension()?.to_str()?;
        match ext {
            "rs" => Some("rust"),
            "py" | "pyi" => Some("python"),
            "ts" | "tsx" | "js" | "jsx" => Some("typescript"),
            "go" => Some("go"),
            _ => None,
        }
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn manager() -> LspManager {
        LspManager::new(&PathBuf::from("/tmp/test-project"))
    }

    #[test]
    fn new_manager_has_no_connections() {
        let mgr = manager();
        assert_eq!(mgr.active_connections(), 0);
    }

    #[test]
    fn new_manager_has_no_diagnostics() {
        let mgr = manager();
        assert!(mgr.diagnostics().is_empty());
    }

    #[test]
    fn push_diagnostics_replaces_per_file() {
        let mut mgr = manager();
        mgr.push_diagnostics(vec![Diagnostic {
            file: "src/main.rs".to_string(),
            line: 1,
            col: 0,
            severity: DiagnosticSeverity::Error,
            message: "unused import".to_string(),
            source: "rust-analyzer".to_string(),
        }]);
        assert_eq!(mgr.diagnostics().len(), 1);

        // Push new diags for same file — old ones removed.
        mgr.push_diagnostics(vec![Diagnostic {
            file: "src/main.rs".to_string(),
            line: 5,
            col: 2,
            severity: DiagnosticSeverity::Warning,
            message: "dead code".to_string(),
            source: "rust-analyzer".to_string(),
        }]);
        assert_eq!(mgr.diagnostics().len(), 1);
        assert_eq!(mgr.diagnostics()[0].line, 5);
    }

    #[test]
    fn push_diagnostics_accumulates_across_files() {
        let mut mgr = manager();
        mgr.push_diagnostics(vec![Diagnostic {
            file: "src/main.rs".to_string(),
            line: 1,
            col: 0,
            severity: DiagnosticSeverity::Error,
            message: "e1".to_string(),
            source: "rust-analyzer".to_string(),
        }]);
        mgr.push_diagnostics(vec![Diagnostic {
            file: "src/lib.rs".to_string(),
            line: 2,
            col: 0,
            severity: DiagnosticSeverity::Warning,
            message: "w1".to_string(),
            source: "rust-analyzer".to_string(),
        }]);
        assert_eq!(mgr.diagnostics().len(), 2);
    }

    #[test]
    fn stop_all_clears_diagnostics() {
        let mut mgr = manager();
        mgr.push_diagnostics(vec![Diagnostic {
            file: "src/main.rs".to_string(),
            line: 1,
            col: 0,
            severity: DiagnosticSeverity::Error,
            message: "e".to_string(),
            source: "rust-analyzer".to_string(),
        }]);
        mgr.stop_all();
        assert!(mgr.diagnostics().is_empty());
        assert_eq!(mgr.active_connections(), 0);
    }

    #[test]
    fn hover_returns_none_with_no_connections() {
        let mgr = manager();
        assert!(mgr.hover("src/main.rs", 0, 0).is_none());
    }

    #[test]
    fn goto_definition_returns_none_with_no_connections() {
        let mgr = manager();
        assert!(mgr.goto_definition("src/main.rs", 0, 0).is_none());
    }

    #[test]
    fn language_for_file_maps_extensions_correctly() {
        let mgr = manager();
        assert_eq!(mgr.language_for_file("foo.rs"), Some("rust"));
        assert_eq!(mgr.language_for_file("foo.py"), Some("python"));
        assert_eq!(mgr.language_for_file("foo.pyi"), Some("python"));
        assert_eq!(mgr.language_for_file("foo.ts"), Some("typescript"));
        assert_eq!(mgr.language_for_file("foo.tsx"), Some("typescript"));
        assert_eq!(mgr.language_for_file("foo.js"), Some("typescript"));
        assert_eq!(mgr.language_for_file("foo.jsx"), Some("typescript"));
        assert_eq!(mgr.language_for_file("foo.go"), Some("go"));
        assert_eq!(mgr.language_for_file("foo.txt"), None);
        assert_eq!(mgr.language_for_file("Makefile"), None);
    }

    #[test]
    fn diagnostic_severity_labels() {
        assert_eq!(DiagnosticSeverity::Error.label(), "ERROR");
        assert_eq!(DiagnosticSeverity::Warning.label(), "WARN");
        assert_eq!(DiagnosticSeverity::Information.label(), "INFO");
        assert_eq!(DiagnosticSeverity::Hint.label(), "HINT");
    }
}
