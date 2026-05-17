use std::fs;
use std::path::PathBuf;

/// OpenCode plugin installer.
/// Copies `mindisland-opencode-plugin.js` to `~/.config/opencode/plugins/`.
/// The plugin runs inside OpenCode and forwards events to MindIsland's socket.
pub struct OpenCodeInstaller {
    source_path: PathBuf,
    target_dir: PathBuf,
}

impl OpenCodeInstaller {
    pub fn new(source_path: String) -> Self {
        let target_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/opencode/plugins");
        Self {
            source_path: PathBuf::from(source_path),
            target_dir,
        }
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
        target.exists()
    }

    /// Install the plugin by copying to OpenCode's plugins directory.
    pub fn install(&self) -> Result<(), String> {
        if !self.source_path.exists() {
            return Err(format!(
                "Plugin source not found: {:?}",
                self.source_path
            ));
        }

        // Ensure plugins directory exists
        fs::create_dir_all(&self.target_dir)
            .map_err(|e| format!("Failed to create plugins dir: {}", e))?;

        let target = self.target_dir.join("mindisland.js");

        // Check if update needed (compare content)
        let source_content = fs::read_to_string(&self.source_path)
            .map_err(|e| format!("Failed to read source: {}", e))?;
        if target.exists() {
            if let Ok(existing) = fs::read_to_string(&target) {
                if existing == source_content {
                    return Ok(()); // Already up to date
                }
            }
        }

        fs::write(&target, source_content)
            .map_err(|e| format!("Failed to write plugin: {}", e))?;

        eprintln!(
            "[mindisland] Installed OpenCode plugin at {:?}",
            target
        );
        Ok(())
    }

    /// Uninstall the plugin.
    pub fn uninstall(&self) -> Result<(), String> {
        let target = self.target_dir.join("mindisland.js");
        if target.exists() {
            fs::remove_file(&target)
                .map_err(|e| format!("Failed to remove plugin: {}", e))?;
            eprintln!("[mindisland] Uninstalled OpenCode plugin");
        }
        Ok(())
    }
}
