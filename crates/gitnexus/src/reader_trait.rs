//! Trait abstracting `GitNexus` backends (CLI shell-out vs native file reader).

use crate::cli_runner::GitNexusCliError;
use crate::types::{ContextResult, ImpactResult, QueryResult};

/// Abstraction over `GitNexus` data sources.
///
/// Implemented by `GitNexusClient` (CLI shell-out) and `NativeGitNexusReader`
/// (reads `.gitnexus/` files directly). Use `AutoGitNexusReader` to
/// auto-select the best available backend.
pub trait GitNexusReader {
    /// Whether this backend is usable in the current environment.
    fn is_available(&self) -> bool;

    /// Run impact analysis on a symbol. Returns upstream callers and risk level.
    fn impact(&self, symbol: &str) -> Result<ImpactResult, GitNexusCliError>;

    /// Get 360-degree context for a symbol (callers, callees, processes).
    fn context(&self, symbol: &str) -> Result<ContextResult, GitNexusCliError>;

    /// Semantic code query — find symbols matching a concept string.
    fn query(&self, query: &str) -> Result<QueryResult, GitNexusCliError>;
}
