use std::fs;
use std::path::PathBuf;

use crate::event::AgentSession;

/// Persists sessions to a JSON file so they survive app restarts.
/// Aligned with Open Vibe Island's ClaudeSessionRegistry.
pub struct SessionRegistry {
    file_path: PathBuf,
}

impl SessionRegistry {
    pub fn new() -> Self {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
            .join("mindisland");
        let _ = fs::create_dir_all(&dir);
        Self {
            file_path: dir.join("session-registry.json"),
        }
    }

    pub fn load(&self) -> Vec<AgentSession> {
        let data = match fs::read_to_string(&self.file_path) {
            Ok(d) => d,
            Err(_) => return vec![],
        };
        serde_json::from_str(&data).unwrap_or_default()
    }

    pub fn save(&self, sessions: &[AgentSession]) {
        if let Ok(data) = serde_json::to_string_pretty(sessions) {
            let _ = fs::write(&self.file_path, data);
        }
    }
}
