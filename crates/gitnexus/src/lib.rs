//! `GitNexus` integration — shell-out to `npx gitnexus` for code intelligence.
//!
//! Provides impact analysis, context lookup, and semantic code queries.
//! Gracefully degrades when `GitNexus` is not installed.

pub mod auto_reader;
pub mod cli_runner;
pub mod native_reader;
pub mod reader_trait;
pub mod types;

use std::path::{Path, PathBuf};
use std::time::Duration;

pub use auto_reader::AutoGitNexusReader;
pub use cli_runner::{is_gitnexus_available, GitNexusCliError};
pub use native_reader::NativeGitNexusReader;
pub use reader_trait::GitNexusReader;
pub use types::{CallerInfo, ContextResult, ImpactResult, QueryMatch, QueryResult, RiskLevel};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Client for `GitNexus` CLI operations.
pub struct GitNexusClient {
    project_root: PathBuf,
    available: bool,
    timeout: Duration,
}

impl GitNexusClient {
    /// Create a new client. Checks availability on construction.
    #[must_use]
    pub fn new(project_root: &Path) -> Self {
        let available = is_gitnexus_available(project_root);
        Self {
            project_root: project_root.to_path_buf(),
            available,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Whether `GitNexus` CLI is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Run impact analysis on a symbol. Returns upstream callers and risk level.
    pub fn impact(&self, symbol: &str) -> Result<ImpactResult, GitNexusCliError> {
        if !self.available {
            return Ok(ImpactResult {
                symbol: symbol.to_string(),
                risk_level: RiskLevel::Low,
                callers: vec![],
                affected_files: vec![],
                raw_output: "GitNexus not available — skipping impact analysis".to_string(),
            });
        }

        let output = cli_runner::run_gitnexus_command(
            &self.project_root,
            &[
                "impact",
                "--target",
                symbol,
                "--direction",
                "upstream",
                "--json",
            ],
            self.timeout,
        )?;

        Ok(parse_impact_output(symbol, &output))
    }

    /// Get 360-degree context for a symbol (callers, callees, processes).
    pub fn context(&self, symbol: &str) -> Result<ContextResult, GitNexusCliError> {
        if !self.available {
            return Ok(ContextResult {
                symbol: symbol.to_string(),
                callers: vec![],
                callees: vec![],
                processes: vec![],
                raw_output: "gitnexus not available".to_string(),
            });
        }

        let output = cli_runner::run_gitnexus_command(
            &self.project_root,
            &["context", "--name", symbol, "--json"],
            self.timeout,
        )?;

        Ok(ContextResult {
            symbol: symbol.to_string(),
            callers: vec![],
            callees: vec![],
            processes: vec![],
            raw_output: output,
        })
    }

    /// Semantic code query — find execution flows related to a concept.
    pub fn query(&self, query: &str) -> Result<QueryResult, GitNexusCliError> {
        if !self.available {
            return Ok(QueryResult {
                query: query.to_string(),
                matches: vec![],
                raw_output: "gitnexus not available".to_string(),
            });
        }

        let output = cli_runner::run_gitnexus_command(
            &self.project_root,
            &["query", "--query", query, "--json"],
            self.timeout,
        )?;

        Ok(QueryResult {
            query: query.to_string(),
            matches: vec![],
            raw_output: output,
        })
    }
}

impl GitNexusReader for GitNexusClient {
    fn is_available(&self) -> bool {
        self.is_available()
    }

    fn impact(&self, symbol: &str) -> Result<ImpactResult, GitNexusCliError> {
        self.impact(symbol)
    }

    fn context(&self, symbol: &str) -> Result<ContextResult, GitNexusCliError> {
        self.context(symbol)
    }

    fn query(&self, query: &str) -> Result<QueryResult, GitNexusCliError> {
        self.query(query)
    }
}

/// Parse impact analysis JSON output.
fn parse_impact_output(symbol: &str, output: &str) -> ImpactResult {
    // Try to parse as JSON; fall back to raw text
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        let risk_str = json
            .get("risk_level")
            .or_else(|| json.get("riskLevel"))
            .and_then(|v| v.as_str())
            .unwrap_or("medium");
        let callers = json
            .get("callers")
            .and_then(|v| serde_json::from_value::<Vec<CallerInfo>>(v.clone()).ok())
            .unwrap_or_default();
        let affected_files = json
            .get("affected_files")
            .or_else(|| json.get("affectedFiles"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        ImpactResult {
            symbol: symbol.to_string(),
            risk_level: RiskLevel::from_str_loose(risk_str),
            callers,
            affected_files,
            raw_output: output.to_string(),
        }
    } else {
        // Fallback: treat as unknown, medium risk
        ImpactResult {
            symbol: symbol.to_string(),
            risk_level: RiskLevel::Medium,
            callers: vec![],
            affected_files: vec![],
            raw_output: output.to_string(),
        }
    }
}
