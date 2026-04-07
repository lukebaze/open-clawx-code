//! Multi-agent team — coordinator + worker agents with message passing.
//!
//! Each agent has an independent orchestrator context. The coordinator
//! decomposes tasks and assigns them to workers via a message bus.

use std::sync::mpsc;

/// Maximum number of concurrent agents (memory bound).
const MAX_AGENTS: usize = 5;

/// Status of an individual agent in the team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Working { task: String },
    WaitingForPeer { peer_id: String },
    Done { summary: String },
    Failed { error: String },
}

impl AgentStatus {
    /// Short label for TUI display.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Idle => "IDLE",
            Self::Working { .. } => "WORKING",
            Self::WaitingForPeer { .. } => "WAITING",
            Self::Done { .. } => "DONE",
            Self::Failed { .. } => "FAILED",
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Failed { .. })
    }
}

/// An individual agent context within a team.
pub struct AgentContext {
    pub id: String,
    pub name: String,
    pub model: String,
    pub status: AgentStatus,
    pub current_task: Option<String>,
    pub messages_sent: usize,
    message_tx: mpsc::Sender<AgentMessage>,
}

impl AgentContext {
    /// Send a message to the message bus.
    pub fn send_message(&mut self, to: String, content: MessageContent) {
        let msg = AgentMessage {
            from: self.id.clone(),
            to,
            content,
        };
        let _ = self.message_tx.send(msg);
        self.messages_sent += 1;
    }

    /// Update agent status.
    pub fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
    }
}

/// Message passed between agents via the message bus.
#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub from: String,
    pub to: String, // agent id or "broadcast"
    pub content: MessageContent,
}

/// Content types for inter-agent messages.
#[derive(Debug, Clone)]
pub enum MessageContent {
    TaskAssignment {
        description: String,
        files: Vec<String>,
    },
    TaskResult {
        summary: String,
        files_modified: Vec<String>,
    },
    Question(String),
    Answer(String),
}

/// Events emitted by the agent team to the TUI.
#[derive(Debug, Clone)]
pub enum AgentTeamEvent {
    /// Agent status changed.
    AgentStatusChanged {
        agent_id: String,
        status: AgentStatus,
    },
    /// Message sent between agents.
    MessageSent(AgentMessage),
    /// All agents completed their tasks.
    TeamDone { summary: String },
    /// Team encountered an unrecoverable error.
    TeamFailed { error: String },
}

/// Multi-agent team with a coordinator and worker agents.
pub struct AgentTeam {
    coordinator: AgentContext,
    workers: Vec<AgentContext>,
    message_rx: mpsc::Receiver<AgentMessage>,
    message_tx: mpsc::Sender<AgentMessage>,
    event_tx: mpsc::Sender<AgentTeamEvent>,
    pending_messages: Vec<AgentMessage>,
}

impl AgentTeam {
    /// Create a new agent team with a coordinator.
    #[must_use]
    pub fn new(
        coordinator_model: String,
        event_tx: mpsc::Sender<AgentTeamEvent>,
    ) -> Self {
        let (message_tx, message_rx) = mpsc::channel();

        let coordinator = AgentContext {
            id: "coordinator".to_string(),
            name: "Coordinator".to_string(),
            model: coordinator_model,
            status: AgentStatus::Idle,
            current_task: None,
            messages_sent: 0,
            message_tx: message_tx.clone(),
        };

        Self {
            coordinator,
            workers: Vec::new(),
            message_rx,
            message_tx,
            event_tx,
            pending_messages: Vec::new(),
        }
    }

    /// Add a worker agent to the team.
    /// Returns the worker's ID, or None if at max capacity.
    pub fn add_worker(&mut self, name: String, model: String) -> Option<String> {
        if self.workers.len() >= MAX_AGENTS - 1 {
            return None; // -1 for coordinator
        }

        let id = format!("worker-{}", self.workers.len());
        let worker = AgentContext {
            id: id.clone(),
            name,
            model,
            status: AgentStatus::Idle,
            current_task: None,
            messages_sent: 0,
            message_tx: self.message_tx.clone(),
        };
        self.workers.push(worker);
        Some(id)
    }

    /// Get a read-only view of the coordinator.
    #[must_use]
    pub fn coordinator(&self) -> &AgentContext {
        &self.coordinator
    }

    /// Get a read-only view of all workers.
    #[must_use]
    pub fn workers(&self) -> &[AgentContext] {
        &self.workers
    }

    /// Total number of agents (coordinator + workers).
    #[must_use]
    pub fn agent_count(&self) -> usize {
        1 + self.workers.len()
    }

    /// Drain pending messages from the bus and route them.
    pub fn process_messages(&mut self) {
        while let Ok(msg) = self.message_rx.try_recv() {
            let _ = self.event_tx.send(AgentTeamEvent::MessageSent(msg.clone()));
            // All messages go to pending — routed when taken by recipient
            self.pending_messages.push(msg);
        }
    }

    /// Take pending messages for a specific agent.
    pub fn take_messages_for(&mut self, agent_id: &str) -> Vec<AgentMessage> {
        let (matching, remaining): (Vec<_>, Vec<_>) = self
            .pending_messages
            .drain(..)
            .partition(|m| m.to == agent_id || m.to == "broadcast");
        self.pending_messages = remaining;
        matching
    }

    /// Assign a task to a specific worker via the coordinator.
    pub fn assign_task(&mut self, worker_id: &str, description: String, files: Vec<String>) {
        self.coordinator.send_message(
            worker_id.to_string(),
            MessageContent::TaskAssignment {
                description: description.clone(),
                files,
            },
        );

        if let Some(worker) = self.workers.iter_mut().find(|w| w.id == worker_id) {
            worker.status = AgentStatus::Working {
                task: description.clone(),
            };
            worker.current_task = Some(description);
            let _ = self.event_tx.send(AgentTeamEvent::AgentStatusChanged {
                agent_id: worker_id.to_string(),
                status: worker.status.clone(),
            });
        }
    }

    /// Mark a worker as done and check if all workers are complete.
    pub fn mark_worker_done(&mut self, worker_id: &str, summary: &str) {
        if let Some(worker) = self.workers.iter_mut().find(|w| w.id == worker_id) {
            worker.status = AgentStatus::Done {
                summary: summary.to_string(),
            };
            worker.current_task = None;
            let _ = self.event_tx.send(AgentTeamEvent::AgentStatusChanged {
                agent_id: worker_id.to_string(),
                status: worker.status.clone(),
            });
        }

        if self.workers.iter().all(|w| w.status.is_terminal()) {
            let team_summary = self
                .workers
                .iter()
                .filter_map(|w| match &w.status {
                    AgentStatus::Done { summary } => Some(format!("{}: {summary}", w.name)),
                    AgentStatus::Failed { error } => Some(format!("{}: FAILED — {error}", w.name)),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("; ");

            let _ = self.event_tx.send(AgentTeamEvent::TeamDone {
                summary: team_summary,
            });
        }
    }

    /// Mark a worker as failed.
    pub fn mark_worker_failed(&mut self, worker_id: &str, error: &str) {
        if let Some(worker) = self.workers.iter_mut().find(|w| w.id == worker_id) {
            worker.status = AgentStatus::Failed {
                error: error.to_string(),
            };
            worker.current_task = None;
            let _ = self.event_tx.send(AgentTeamEvent::AgentStatusChanged {
                agent_id: worker_id.to_string(),
                status: worker.status.clone(),
            });
        }
    }
}
