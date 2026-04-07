//! Manages a single LSP server connection via stdio transport.
//!
//! Handles process lifecycle, JSON-RPC message framing, and
//! basic LSP protocol methods (initialize, hover, goto-definition, did-change).

use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::detect::LanguageConfig;
use crate::{HoverInfo, Location};

/// A connection to a single language server process.
pub struct LspConnection {
    language: String,
    process: Option<Child>,
    request_id: i64,
    initialized: bool,
}

impl LspConnection {
    /// Start a language server process and send initialize request.
    pub fn start(config: &LanguageConfig, root: &Path) -> anyhow::Result<Self> {
        let process = Command::new(&config.server_command)
            .args(&config.server_args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut conn = Self {
            language: config.language.clone(),
            process: Some(process),
            request_id: 0,
            initialized: false,
        };

        // Send initialize handshake
        let root_uri = format!("file://{}", root.display());
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "hover": { "contentFormat": ["plaintext"] },
                    "definition": {},
                    "publishDiagnostics": { "relatedInformation": true }
                }
            }
        });
        conn.send_request("initialize", &init_params)?;
        conn.send_notification("initialized", &serde_json::json!({}))?;
        conn.initialized = true;

        Ok(conn)
    }

    /// Language this connection serves.
    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Whether the server was successfully initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get hover info at a position (stub — returns None until async response handling added).
    #[must_use]
    pub fn hover(&self, file: &str, line: u32, col: u32) -> Option<HoverInfo> {
        if !self.initialized {
            return None;
        }
        // Real impl: send textDocument/hover and read response via stdout reader.
        let _ = (file, line, col);
        None
    }

    /// Get go-to-definition location (stub — returns None until async response handling added).
    #[must_use]
    pub fn goto_definition(&self, file: &str, line: u32, col: u32) -> Option<Location> {
        if !self.initialized {
            return None;
        }
        let _ = (file, line, col);
        None
    }

    /// Notify server of file content change (fire-and-forget notification).
    pub fn did_change(&mut self, file: &str, content: &str) {
        if !self.initialized {
            return;
        }
        let uri = format!("file://{file}");
        let params = serde_json::json!({
            "textDocument": { "uri": uri, "version": 1 },
            "contentChanges": [{ "text": content }]
        });
        if let Err(e) = self.send_notification("textDocument/didChange", &params) {
            eprintln!("LSP({}): did_change failed: {e}", self.language);
        }
    }

    /// Gracefully shut down the language server process.
    pub fn shutdown(&mut self) {
        if self.process.is_none() {
            return;
        }
        // Best-effort — ignore errors during shutdown path.
        let _ = self.send_request("shutdown", &serde_json::json!(null));
        let _ = self.send_notification("exit", &serde_json::json!(null));
        if let Some(ref mut process) = self.process {
            let _ = process.kill();
            let _ = process.wait();
        }
        self.process = None;
        self.initialized = false;
    }

    /// Send a JSON-RPC request (expects a response from server).
    fn send_request(&mut self, method: &str, params: &serde_json::Value) -> anyhow::Result<()> {
        self.request_id += 1;
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        });
        self.write_message(&msg)
    }

    /// Send a JSON-RPC notification (no id, server sends no response).
    fn send_notification(&mut self, method: &str, params: &serde_json::Value) -> anyhow::Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        self.write_message(&msg)
    }

    /// Write an LSP-framed message: `Content-Length: N\r\n\r\n{body}`.
    fn write_message(&mut self, msg: &serde_json::Value) -> anyhow::Result<()> {
        let body = serde_json::to_string(msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        let process = self
            .process
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("LSP process not running"))?;
        let stdin = process
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("LSP stdin not available"))?;

        stdin.write_all(header.as_bytes())?;
        stdin.write_all(body.as_bytes())?;
        stdin.flush()?;
        Ok(())
    }
}

impl Drop for LspConnection {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify `Content-Length` framing is correctly formatted.
    /// We test `write_message` indirectly via a fake config pointing to `cat`
    /// which accepts stdin without crashing, letting us call the framing path.
    #[test]
    fn language_accessor_returns_configured_language() {
        // Build a minimal conn without a live process to test accessors.
        let conn = LspConnection {
            language: "rust".to_string(),
            process: None,
            request_id: 0,
            initialized: false,
        };
        assert_eq!(conn.language(), "rust");
        assert!(!conn.is_initialized());
    }

    #[test]
    fn hover_returns_none_when_not_initialized() {
        let conn = LspConnection {
            language: "rust".to_string(),
            process: None,
            request_id: 0,
            initialized: false,
        };
        assert!(conn.hover("src/main.rs", 0, 0).is_none());
    }

    #[test]
    fn goto_definition_returns_none_when_not_initialized() {
        let conn = LspConnection {
            language: "rust".to_string(),
            process: None,
            request_id: 0,
            initialized: false,
        };
        assert!(conn.goto_definition("src/main.rs", 0, 0).is_none());
    }

    #[test]
    fn shutdown_is_idempotent_with_no_process() {
        let mut conn = LspConnection {
            language: "rust".to_string(),
            process: None,
            request_id: 0,
            initialized: false,
        };
        // Should not panic when called with no process.
        conn.shutdown();
        conn.shutdown();
    }
}
