use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::event::{AgentEvent, SessionPhase};

/// Discover existing Claude Code sessions by scanning JSONL transcript files.
/// Aligned with Open Vibe Island's ClaudeTranscriptDiscovery.
pub struct ClaudeTranscriptDiscovery {
    root: PathBuf,
    max_age: Duration,
    max_files: usize,
}

struct DiscoveredSession {
    session_id: String,
    cwd: String,
    title: String,
    model: Option<String>,
    last_summary: String,
    updated_at: u64,
}

impl ClaudeTranscriptDiscovery {
    pub fn new() -> Self {
        let root = dirs::home_dir()
            .unwrap_or_default()
            .join(".claude/projects");
        Self {
            root,
            max_age: Duration::from_secs(86_400), // 24 hours
            max_files: 20,
        }
    }

    /// Scan transcript files and return SessionStarted + ActivityUpdated events
    /// for each discovered session.
    pub fn discover(&self) -> Vec<AgentEvent> {
        let sessions = self.scan_transcripts();
        let mut events = Vec::new();

        for s in sessions {
            events.push(AgentEvent::SessionStarted {
                agent_id: "claude-code".to_string(),
                session_id: s.session_id.clone(),
                title: s.title,
                directory: s.cwd.clone(),
                model: s.model,
            });
            events.push(AgentEvent::ActivityUpdated {
                session_id: s.session_id,
                phase: SessionPhase::Completed,
                summary: s.last_summary,
                tool_name: None,
            });
        }

        eprintln!(
            "[mindisland] Discovered {} Claude Code sessions from transcripts",
            events.len() / 2
        );
        events
    }

    fn scan_transcripts(&self) -> Vec<DiscoveredSession> {
        if !self.root.exists() {
            return vec![];
        }

        let now = SystemTime::now();
        let cutoff = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - self.max_age.as_millis() as u64;

        // Find all .jsonl files (skip subagents)
        let mut candidates: Vec<(PathBuf, u64)> = Vec::new();
        self.walk_dir(&self.root, &mut candidates);

        // Sort by modification time (newest first), take max_files
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(self.max_files);

        // Filter by age
        candidates.retain(|(_, mtime)| *mtime >= cutoff);

        // Parse each transcript
        candidates
            .into_iter()
            .filter_map(|(path, mtime)| self.parse_transcript(&path, mtime))
            .collect()
    }

    fn walk_dir(&self, dir: &PathBuf, results: &mut Vec<(PathBuf, u64)>) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip subagents directory
                if path.file_name().map(|n| n == "subagents").unwrap_or(false) {
                    continue;
                }
                self.walk_dir(&path, results);
            } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                results.push((path, mtime));
            }
        }
    }

    fn parse_transcript(&self, path: &PathBuf, mtime: u64) -> Option<DiscoveredSession> {
        let file = fs::File::open(path).ok()?;
        let reader = BufReader::new(file);

        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let mut cwd: Option<String> = None;
        let mut model: Option<String> = None;
        let mut initial_prompt: Option<String> = None;
        let mut last_prompt: Option<String> = None;
        let mut last_assistant: Option<String> = None;
        let mut updated_at = mtime;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.is_empty() {
                continue;
            }

            let obj: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Extract cwd
            if let Some(c) = obj.get("cwd").and_then(|v| v.as_str()) {
                if !c.is_empty() {
                    cwd = Some(c.to_string());
                }
            }

            // Extract timestamp
            if let Some(ts) = obj.get("timestamp").and_then(|v| v.as_str()) {
                // ISO 8601 timestamp → unix millis
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                    updated_at = dt.timestamp_millis() as u64;
                }
            }

            // Extract sessionId (some transcripts have it)
            // We already have it from the filename, but sessionId field is authoritative

            // Parse message
            if let Some(message) = obj.get("message") {
                let role = message.get("role").and_then(|v| v.as_str()).unwrap_or("");

                if role == "user" {
                    if let Some(text) = extract_text_content(message.get("content")) {
                        if initial_prompt.is_none() {
                            initial_prompt = Some(text.clone());
                        }
                        last_prompt = Some(text);
                    }
                } else if role == "assistant" {
                    if let Some(m) = message.get("model").and_then(|v| v.as_str()) {
                        if !m.is_empty() {
                            model = Some(m.to_string());
                        }
                    }
                    if let Some(text) = extract_text_content(message.get("content")) {
                        last_assistant = Some(text);
                    }
                }
            }

            // Handle summary type entries
            if obj.get("type").and_then(|v| v.as_str()) == Some("summary") {
                if let Some(s) = obj.get("summary").and_then(|v| v.as_str()) {
                    if !s.is_empty() {
                        last_assistant = Some(clip(s, 140));
                    }
                }
            }
        }

        let cwd = cwd?;
        let workspace = project_name(&cwd);

        let summary = last_assistant
            .or(last_prompt)
            .unwrap_or_else(|| format!("Claude Code session in {}", workspace));

        Some(DiscoveredSession {
            session_id,
            cwd,
            title: workspace,
            model,
            last_summary: summary,
            updated_at,
        })
    }
}

fn extract_text_content(content: Option<&serde_json::Value>) -> Option<String> {
    let content = content?;

    // String content (simple prompt)
    if let Some(text) = content.as_str() {
        return Some(clip(text, 140));
    }

    // Array of content blocks
    if let Some(blocks) = content.as_array() {
        for block in blocks {
            if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        return Some(clip(text, 140));
                    }
                }
            }
        }
    }

    None
}

fn clip(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    let trimmed = first_line.trim();
    if trimmed.chars().count() > max {
        let truncated: String = trimmed.chars().take(max).collect();
        format!("{}...", truncated)
    } else {
        trimmed.to_string()
    }
}

fn project_name(cwd: &str) -> String {
    let parts: Vec<&str> = cwd.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        parts.last().unwrap_or(&"Claude Code").to_string()
    }
}
