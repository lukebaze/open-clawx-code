use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use claw_runtime::{
    session_control::ManagedSessionSummary, session_control::SessionControlError,
    session_control::SessionHandle, Session, SessionStore,
};

/// Data dir under user home for OCX sessions and config.
const APP_DIR_NAME: &str = ".open-clawx-code";

/// Manages session persistence for the TUI.
/// Wraps claw-runtime `SessionStore` with an OCX-specific data directory.
pub struct SessionManager {
    store: SessionStore,
    current_handle: Option<SessionHandle>,
    sessions_dir: PathBuf,
}

/// Lightweight summary for the session picker UI.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub message_count: usize,
    pub modified_epoch_millis: u128,
}

impl SessionManager {
    /// Create a new manager, initializing the sessions directory.
    /// `workspace_root` is typically the current working directory.
    pub fn new(workspace_root: &Path) -> Result<Self, SessionControlError> {
        let data_dir = home_data_dir();
        let store = SessionStore::from_data_dir(&data_dir, workspace_root)?;
        let sessions_dir = store.sessions_dir().to_path_buf();
        Ok(Self {
            store,
            current_handle: None,
            sessions_dir,
        })
    }

    /// List recent sessions, newest first.
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, SessionControlError> {
        let managed = self.store.list_sessions()?;
        Ok(managed.into_iter().map(to_summary).collect())
    }

    /// Load a session by ID or alias ("latest", "last", "recent").
    pub fn load_session(&mut self, reference: &str) -> Result<Session, SessionControlError> {
        let loaded = self.store.load_session(reference)?;
        self.current_handle = Some(loaded.handle);
        Ok(loaded.session)
    }

    /// Create a fresh session bound to the current workspace.
    pub fn create_session(&mut self) -> Result<Session, SessionControlError> {
        let session = Session::new();
        let handle = self.store.create_handle(&session.session_id);
        let session = session.with_persistence_path(&handle.path);
        // Save the empty session to disk so it shows in `list_sessions`
        session.save_to_path(&handle.path)?;
        self.current_handle = Some(handle);
        Ok(session)
    }

    /// Save the given session to its persistence path.
    pub fn save_session(session: &Session) -> Result<(), SessionControlError> {
        if let Some(path) = session.persistence_path() {
            session.save_to_path(path)?;
        }
        Ok(())
    }

    /// Whether previous sessions exist (for startup picker decision).
    #[must_use]
    pub fn has_sessions(&self) -> bool {
        self.store
            .list_sessions()
            .map(|list| !list.is_empty())
            .unwrap_or(false)
    }

    #[must_use]
    pub fn current_handle(&self) -> Option<&SessionHandle> {
        self.current_handle.as_ref()
    }

    #[must_use]
    pub fn sessions_dir(&self) -> &Path {
        &self.sessions_dir
    }
}

fn to_summary(managed: ManagedSessionSummary) -> SessionSummary {
    // Title: use session ID truncated (first user message not available in summary)
    let title = if managed.id.len() > 12 {
        format!("{}…", &managed.id[..12])
    } else {
        managed.id.clone()
    };
    SessionSummary {
        id: managed.id,
        title,
        message_count: managed.message_count,
        modified_epoch_millis: managed.modified_epoch_millis,
    }
}

/// Resolve the OCX data directory: `$HOME/.open-clawx-code/`
fn home_data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(APP_DIR_NAME)
    } else {
        // Fallback: use current dir (shouldn't happen on macOS/Linux)
        PathBuf::from(".").join(APP_DIR_NAME)
    }
}

/// Format epoch millis into a human-readable relative time string.
#[must_use]
pub fn format_relative_time(epoch_millis: u128) -> String {
    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let diff_secs = now_millis.saturating_sub(epoch_millis) / 1000;
    if diff_secs < 60 {
        "just now".to_string()
    } else if diff_secs < 3600 {
        format!("{}m ago", diff_secs / 60)
    } else if diff_secs < 86400 {
        format!("{}h ago", diff_secs / 3600)
    } else {
        format!("{}d ago", diff_secs / 86400)
    }
}
