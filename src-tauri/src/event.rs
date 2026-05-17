use serde::{Deserialize, Serialize};

/// Session phase — aligned with Open Vibe Island's SessionPhase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionPhase {
    Running,
    WaitingForApproval,
    WaitingForAnswer,
    Completed,
}

/// A unified agent session visible to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub brand_color: String,
    pub title: String,
    pub directory: String,
    pub phase: SessionPhase,
    pub summary: String,
    pub updated_at: u64,
    pub model: Option<String>,
    pub current_tool: Option<String>,
    pub pending_permission: Option<PermissionRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub tool_name: Option<String>,
}

/// Internal events emitted by agent adapters
#[derive(Debug, Clone)]
pub enum AgentEvent {
    SessionStarted {
        agent_id: String,
        session_id: String,
        title: String,
        directory: String,
        model: Option<String>,
    },
    ActivityUpdated {
        session_id: String,
        phase: SessionPhase,
        summary: String,
        tool_name: Option<String>,
    },
    PermissionRequested {
        session_id: String,
        permission: PermissionRequest,
    },
    QuestionAsked {
        session_id: String,
        question: String,
    },
    SessionCompleted {
        session_id: String,
        summary: String,
    },
    SessionEnded {
        session_id: String,
    },
}
