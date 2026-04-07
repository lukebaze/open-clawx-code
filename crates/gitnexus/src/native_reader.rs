//! Native `.gitnexus/` directory reader — no CLI shell-out required.
//!
//! Reads `graph.json` directly from the `.gitnexus/` directory to answer
//! impact, context, and query requests without spawning `npx gitnexus`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::cli_runner::GitNexusCliError;
use crate::reader_trait::GitNexusReader;
use crate::types::{CallerInfo, ContextResult, ImpactResult, QueryMatch, QueryResult, RiskLevel};

/// Reads `.gitnexus/graph.json` directly (no CLI shell-out).
pub struct NativeGitNexusReader {
    /// Retained for future cache-invalidation checks (mtime comparison).
    #[allow(dead_code)]
    graph_path: PathBuf,
    graph: Option<GraphData>,
}

#[derive(Debug, Deserialize)]
struct GraphData {
    #[serde(default)]
    symbols: Vec<SymbolNode>,
    #[serde(default)]
    relationships: Vec<Relationship>,
}

#[derive(Debug, Deserialize)]
struct SymbolNode {
    name: String,
    #[serde(default)]
    file: String,
    /// Retained for future kind-based filtering (function, class, method, etc.).
    #[allow(dead_code)]
    #[serde(default, rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct Relationship {
    from: String,
    to: String,
    #[serde(default, rename = "type")]
    kind: String,
}

impl NativeGitNexusReader {
    /// Load graph data from `<project_root>/.gitnexus/graph.json`.
    /// Never fails — if the file is absent or malformed, `is_available()` returns false.
    #[must_use]
    pub fn load(project_root: &Path) -> Self {
        let graph_path = project_root.join(".gitnexus");
        let graph = Self::try_load_graph(&graph_path);
        Self { graph_path, graph }
    }

    fn try_load_graph(path: &Path) -> Option<GraphData> {
        let graph_file = path.join("graph.json");
        let content = std::fs::read_to_string(graph_file).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Resolve the file path of a symbol by name from the graph.
    fn symbol_file<'g>(graph: &'g GraphData, name: &str) -> &'g str {
        graph
            .symbols
            .iter()
            .find(|s| s.name == name)
            .map_or("", |s| s.file.as_str())
    }

    fn risk_from_caller_count(count: usize) -> RiskLevel {
        match count {
            0..=2 => RiskLevel::Low,
            3..=5 => RiskLevel::Medium,
            6..=10 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

impl GitNexusReader for NativeGitNexusReader {
    fn is_available(&self) -> bool {
        self.graph.is_some()
    }

    fn impact(&self, symbol: &str) -> Result<ImpactResult, GitNexusCliError> {
        let Some(graph) = &self.graph else {
            return Ok(ImpactResult {
                symbol: symbol.to_string(),
                risk_level: RiskLevel::Low,
                callers: vec![],
                affected_files: vec![],
                raw_output: "Native reader: no graph loaded".to_string(),
            });
        };

        // Upstream callers: relationships where `to == symbol` and kind == "calls"
        let callers: Vec<CallerInfo> = graph
            .relationships
            .iter()
            .filter(|r| r.to == symbol && r.kind == "calls")
            .map(|r| CallerInfo {
                file: Self::symbol_file(graph, &r.from).to_string(),
                name: r.from.clone(),
                depth: 1,
            })
            .collect();

        let affected_files: Vec<String> = callers
            .iter()
            .map(|c| c.file.clone())
            .filter(|f| !f.is_empty())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let risk_level = Self::risk_from_caller_count(callers.len());
        let caller_count = callers.len();

        Ok(ImpactResult {
            symbol: symbol.to_string(),
            risk_level,
            callers,
            affected_files,
            raw_output: format!("Native reader: {caller_count} callers found"),
        })
    }

    fn context(&self, symbol: &str) -> Result<ContextResult, GitNexusCliError> {
        let Some(graph) = &self.graph else {
            return Ok(ContextResult {
                symbol: symbol.to_string(),
                callers: vec![],
                callees: vec![],
                processes: vec![],
                raw_output: "Native reader: no graph loaded".to_string(),
            });
        };

        let incoming: Vec<String> = graph
            .relationships
            .iter()
            .filter(|r| r.to == symbol)
            .map(|r| r.from.clone())
            .collect();

        let outgoing: Vec<String> = graph
            .relationships
            .iter()
            .filter(|r| r.from == symbol)
            .map(|r| r.to.clone())
            .collect();

        let incoming_count = incoming.len();
        let outgoing_count = outgoing.len();

        Ok(ContextResult {
            symbol: symbol.to_string(),
            callers: incoming,
            callees: outgoing,
            processes: vec![],
            raw_output: format!(
                "Native reader: {incoming_count} callers, {outgoing_count} callees"
            ),
        })
    }

    fn query(&self, query: &str) -> Result<QueryResult, GitNexusCliError> {
        let Some(graph) = &self.graph else {
            return Ok(QueryResult {
                query: query.to_string(),
                matches: vec![],
                raw_output: "Native reader: no graph loaded".to_string(),
            });
        };

        let query_lower = query.to_lowercase();
        let matches: Vec<QueryMatch> = graph
            .symbols
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&query_lower))
            .map(|s| QueryMatch {
                symbol: s.name.clone(),
                file: s.file.clone(),
                relevance: 1.0,
            })
            .collect();

        let match_count = matches.len();

        Ok(QueryResult {
            query: query.to_string(),
            matches,
            raw_output: format!("Native reader: {match_count} matches"),
        })
    }
}
