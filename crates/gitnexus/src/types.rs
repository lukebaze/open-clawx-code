use serde::Deserialize;

/// Risk level from `GitNexus` impact analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    /// Parse from string (case-insensitive).
    #[must_use]
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            "critical" | "crit" => Self::Critical,
            _ => Self::Medium,
        }
    }

    /// Short label for TUI display.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MED",
            Self::High => "HIGH",
            Self::Critical => "CRIT",
        }
    }

    /// Whether this risk level requires user approval before proceeding.
    #[must_use]
    pub fn requires_approval(self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

/// A caller/dependent discovered by impact analysis.
#[derive(Debug, Clone, Deserialize)]
pub struct CallerInfo {
    pub name: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub depth: u8,
}

/// Result of `gitnexus impact <symbol>`.
#[derive(Debug, Clone)]
pub struct ImpactResult {
    pub symbol: String,
    pub risk_level: RiskLevel,
    pub callers: Vec<CallerInfo>,
    pub affected_files: Vec<String>,
    pub raw_output: String,
}

/// Result of `gitnexus context <symbol>`.
#[derive(Debug, Clone)]
pub struct ContextResult {
    pub symbol: String,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
    pub processes: Vec<String>,
    pub raw_output: String,
}

/// Result of `gitnexus query <query>`.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub query: String,
    pub matches: Vec<QueryMatch>,
    pub raw_output: String,
}

/// A single match from a `GitNexus` query.
#[derive(Debug, Clone)]
pub struct QueryMatch {
    pub symbol: String,
    pub file: String,
    pub relevance: f32,
}
