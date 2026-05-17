use std::fs;
use std::path::PathBuf;

/// OpenCode plugin installer.
/// Writes embedded plugin content to `~/.config/opencode/plugins/mindisland.js`.
pub struct OpenCodeInstaller {
    content: String,
    target_dir: PathBuf,
}

impl OpenCodeInstaller {
    /// Create from embedded plugin content (no external file dependency).
    pub fn new_from_content(content: String) -> Self {
        let target_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/opencode/plugins");
        Self { content, target_dir }
    }

    /// Check if OpenCode is installed on this system.
    pub fn is_available() -> bool {
        dirs::home_dir()
            .map(|h| h.join(".config/opencode").exists())
            .unwrap_or(false)
    }

    /// Check if MindIsland plugin is already installed in OpenCode.
    pub fn is_installed(&self) -> bool {
        let target = self.target_dir.join("mindisland.js");
        if !target.exists() {
            return false;
        }
        // Check if content matches (auto-update on version change)
        fs::read_to_string(&target)
            .map(|existing| existing == self.content)
            .unwrap_or(false)
    }

    /// Install the plugin by writing to OpenCode's plugins directory.
    pub fn install(&self) -> Result<(), String> {
        fs::create_dir_all(&self.target_dir)
            .map_err(|e| format!("Failed to create plugins dir: {}", e))?;

        let target = self.target_dir.join("mindisland.js");
        fs::write(&target, &self.content)
            .map_err(|e| format!("Failed to write plugin: {}", e))?;

        eprintln!("[mindisland] Installed OpenCode plugin at {:?}", target);
        Ok(())
    }
}
