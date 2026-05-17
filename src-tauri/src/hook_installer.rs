use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// Manages MindIsland hook installation in Claude Code's settings.json.
pub struct HookInstaller {
    settings_path: PathBuf,
    hook_command: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HookStatus {
    pub installed: bool,
    pub events_registered: usize,
    pub settings_path: String,
    pub hook_command: String,
}

impl HookInstaller {
    pub fn new(hook_command: String) -> Self {
        let settings_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".claude/settings.json");
        Self {
            settings_path,
            hook_command,
        }
    }

    pub fn status(&self) -> HookStatus {
        let config = self.load_config().unwrap_or_default();
        let hooks = config.get("hooks").and_then(|v| v.as_object());
        let count = hooks
            .map(|h| {
                h.values()
                    .filter(|entries| self.has_mindisland_hook(entries))
                    .count()
            })
            .unwrap_or(0);

        HookStatus {
            installed: count > 0,
            events_registered: count,
            settings_path: self.settings_path.display().to_string(),
            hook_command: self.hook_command.clone(),
        }
    }

    pub fn install(&self) -> Result<HookStatus, String> {
        let mut config = self.load_config().unwrap_or_else(|| json!({}));
        let hooks = config
            .as_object_mut()
            .ok_or("Invalid settings.json")?
            .entry("hooks")
            .or_insert_with(|| json!({}));

        let events = [
            ("SessionStart", false, false),
            ("UserPromptSubmit", false, false),
            ("PreToolUse", true, false),
            ("PostToolUse", true, false),
            ("PostToolUseFailure", true, false),
            ("PermissionRequest", true, true), // needs timeout
            ("PermissionDenied", true, false),
            ("Notification", true, false),
            ("PreCompact", false, false),
            ("Stop", false, false),
            ("StopFailure", false, false),
            ("SessionEnd", false, false),
            ("SubagentStart", false, false),
            ("SubagentStop", false, false),
        ];

        for (event, needs_matcher, needs_timeout) in events {
            let entries = hooks
                .as_object_mut()
                .ok_or("Invalid hooks")?
                .entry(event)
                .or_insert_with(|| json!([]));

            if self.has_mindisland_hook(entries) {
                continue;
            }

            let mut hook = json!({"type": "command", "command": self.hook_command});
            if needs_timeout {
                hook["timeout"] = json!(86400);
            }

            let mut entry = json!({"hooks": [hook]});
            if needs_matcher {
                entry["matcher"] = json!("*");
            }

            entries
                .as_array_mut()
                .ok_or("Invalid hook entries")?
                .push(entry);
        }

        self.save_config(&config)?;
        Ok(self.status())
    }

    pub fn uninstall(&self) -> Result<HookStatus, String> {
        let mut config = self.load_config().ok_or("No settings.json found")?;

        if let Some(hooks) = config.get_mut("hooks").and_then(|v| v.as_object_mut()) {
            for entries in hooks.values_mut() {
                if let Some(arr) = entries.as_array_mut() {
                    arr.retain(|entry| {
                        !entry
                            .get("hooks")
                            .and_then(|h| h.as_array())
                            .map(|hooks| {
                                hooks.iter().any(|h| {
                                    h.get("command")
                                        .and_then(|c| c.as_str())
                                        .map(|c| c.contains("mindisland"))
                                        .unwrap_or(false)
                                })
                            })
                            .unwrap_or(false)
                    });
                }
            }

            // Clean up empty event arrays
            hooks.retain(|_, v| {
                v.as_array().map(|a| !a.is_empty()).unwrap_or(true)
            });
        }

        self.save_config(&config)?;
        Ok(self.status())
    }

    fn load_config(&self) -> Option<Value> {
        let data = fs::read_to_string(&self.settings_path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_config(&self, config: &Value) -> Result<(), String> {
        // Backup existing file
        if self.settings_path.exists() {
            let backup = self.settings_path.with_extension("json.bak");
            let _ = fs::copy(&self.settings_path, &backup);
        }

        let data = serde_json::to_string_pretty(config)
            .map_err(|e| format!("JSON serialize error: {}", e))?;
        fs::write(&self.settings_path, data)
            .map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    fn has_mindisland_hook(&self, entries: &Value) -> bool {
        entries
            .as_array()
            .map(|arr| {
                arr.iter().any(|entry| {
                    entry
                        .get("hooks")
                        .and_then(|h| h.as_array())
                        .map(|hooks| {
                            hooks.iter().any(|h| {
                                h.get("command")
                                    .and_then(|c| c.as_str())
                                    .map(|c| c.contains("mindisland"))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }
}
