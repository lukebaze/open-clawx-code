//! Auto-selecting `GitNexus` reader.
//!
//! Prefers `NativeGitNexusReader` when `.gitnexus/graph.json` is present,
//! falls back to `GitNexusClient` (CLI shell-out) otherwise.

use std::path::Path;

use crate::cli_runner::GitNexusCliError;
use crate::native_reader::NativeGitNexusReader;
use crate::reader_trait::GitNexusReader;
use crate::types::{ContextResult, ImpactResult, QueryResult};
use crate::GitNexusClient;

/// Auto-selecting reader: uses native file reader if `.gitnexus/` exists,
/// falls back to CLI shell-out via `GitNexusClient`.
pub enum AutoGitNexusReader {
    Native(NativeGitNexusReader),
    Cli(GitNexusClient),
}

impl AutoGitNexusReader {
    /// Construct, probing for `.gitnexus/graph.json` at `project_root`.
    #[must_use]
    pub fn new(project_root: &Path) -> Self {
        let native = NativeGitNexusReader::load(project_root);
        if native.is_available() {
            Self::Native(native)
        } else {
            Self::Cli(GitNexusClient::new(project_root))
        }
    }
}

impl GitNexusReader for AutoGitNexusReader {
    fn is_available(&self) -> bool {
        match self {
            Self::Native(r) => r.is_available(),
            Self::Cli(r) => r.is_available(),
        }
    }

    fn impact(&self, symbol: &str) -> Result<ImpactResult, GitNexusCliError> {
        match self {
            Self::Native(r) => r.impact(symbol),
            Self::Cli(r) => r.impact(symbol),
        }
    }

    fn context(&self, symbol: &str) -> Result<ContextResult, GitNexusCliError> {
        match self {
            Self::Native(r) => r.context(symbol),
            Self::Cli(r) => r.context(symbol),
        }
    }

    fn query(&self, query: &str) -> Result<QueryResult, GitNexusCliError> {
        match self {
            Self::Native(r) => r.query(query),
            Self::Cli(r) => r.query(query),
        }
    }
}
