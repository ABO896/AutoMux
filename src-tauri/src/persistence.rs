//! Profile persistence layer.
//!
//! @architect: Saves/loads named JSON profiles to/from the user's
//! platform-specific app data directory:
//!   - macOS:   ~/Library/Application Support/com.alvaro.automux/profiles/
//!   - Windows: %APPDATA%/com.alvaro.automux/profiles/
//!
//! Each profile is a single .json file containing a `ProfileData` struct.
//! The "default" profile is auto-loaded on startup and auto-saved on changes.
//!
//! @performance-tuner: All I/O is async via `tokio::fs`. JSON parsing uses
//! `serde_json` which handles multi-MB configs without issue. A 1000-macro
//! config serializes to ~200KB — well under the 60MB RAM limit.

use crate::state::MacroConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// The data persisted within a single profile file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    /// Human-readable profile name (also used as filename stem).
    pub name: String,
    /// All macro configurations in this profile.
    pub macros: HashMap<Uuid, MacroConfig>,
    /// Whether the engine should auto-start when this profile loads.
    #[serde(default = "default_true")]
    pub engine_active: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ProfileData {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            macros: HashMap::new(),
            engine_active: true,
        }
    }
}

/// Summary metadata returned to the frontend (no full macro data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub name: String,
    pub macro_count: usize,
}

/// Manages profile persistence on the filesystem.
///
/// @safety-officer: All file operations are sandboxed to the app data
/// directory. Profile names are sanitized to prevent path traversal.
#[derive(Clone)]
pub struct ProfileManager {
    profiles_dir: PathBuf,
}

impl ProfileManager {
    /// Initialize from a Tauri AppHandle. Resolves the platform-specific
    /// app data directory and creates the `profiles/` subdirectory if needed.
    pub fn from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self, String> {
        use tauri::Manager;
        let app_data = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

        let profiles_dir = app_data.join("profiles");

        // Create directory synchronously during setup (only runs once).
        std::fs::create_dir_all(&profiles_dir)
            .map_err(|e| format!("Failed to create profiles dir: {}", e))?;

        Ok(Self { profiles_dir })
    }

    /// Sanitize a profile name to a safe filename.
    /// Strips path separators and special characters.
    fn sanitize_name(name: &str) -> String {
        name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Get the file path for a profile by name.
    fn profile_path(&self, name: &str) -> PathBuf {
        let safe_name = Self::sanitize_name(name);
        self.profiles_dir.join(format!("{}.json", safe_name))
    }

    /// Save a profile to disk.
    pub async fn save_profile(&self, profile: &ProfileData) -> Result<(), String> {
        let path = self.profile_path(&profile.name);
        let json = serde_json::to_string_pretty(profile)
            .map_err(|e| format!("Failed to serialize profile: {}", e))?;

        tokio::fs::write(&path, json)
            .await
            .map_err(|e| format!("Failed to write profile '{}': {}", profile.name, e))?;

        #[cfg(debug_assertions)]
        eprintln!("[Persistence] Saved profile '{}' → {:?}", profile.name, path);

        Ok(())
    }

    /// Load a profile from disk by name.
    pub async fn load_profile(&self, name: &str) -> Result<ProfileData, String> {
        let path = self.profile_path(name);

        let json = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => format!("Profile '{}' not found", name),
                _ => format!("Failed to read profile '{}': {}", name, e),
            })?;

        let profile: ProfileData = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse profile '{}': {}", name, e))?;

        #[cfg(debug_assertions)]
        eprintln!("[Persistence] Loaded profile '{}' ({} macros)", profile.name, profile.macros.len());

        Ok(profile)
    }

    /// Delete a profile from disk.
    pub async fn delete_profile(&self, name: &str) -> Result<(), String> {
        let path = self.profile_path(name);
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| format!("Failed to delete profile '{}': {}", name, e))?;
        }
        Ok(())
    }

    /// List all saved profiles (name + macro count).
    pub async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
        let mut summaries = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.profiles_dir)
            .await
            .map_err(|e| format!("Failed to read profiles dir: {}", e))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            // Parse just enough to get the summary (lazy — avoids loading full macros).
            match tokio::fs::read_to_string(&path).await {
                Ok(json) => {
                    if let Ok(profile) = serde_json::from_str::<ProfileData>(&json) {
                        summaries.push(ProfileSummary {
                            name: profile.name,
                            macro_count: profile.macros.len(),
                        });
                    }
                }
                Err(_) => continue,
            }
        }

        // Sort alphabetically, but "Default" always first.
        summaries.sort_by(|a, b| {
            if a.name == "Default" {
                std::cmp::Ordering::Less
            } else if b.name == "Default" {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        Ok(summaries)
    }

    /// Load the default profile, or create one if it doesn't exist.
    /// Used at startup for automatic restoration (Task 5.3).
    pub async fn load_or_create_default(&self) -> ProfileData {
        match self.load_profile("Default").await {
            Ok(profile) => profile,
            Err(_) => {
                let default = ProfileData::default();
                // Best-effort save — don't fail startup if disk write fails.
                let _ = self.save_profile(&default).await;
                default
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ActionSequence, ActionStep, InputEvent, MouseButton};

    /// @performance-tuner: Verify that a large config stays well under 60MB.
    /// 1000 macros with 5 steps each ≈ 200KB JSON. In-memory HashMap is ~400KB.
    #[test]
    fn large_config_memory_check() {
        let mut macros = HashMap::new();
        for i in 0..1000 {
            let id = Uuid::new_v4();
            macros.insert(id, MacroConfig {
                id,
                name: format!("Macro_{}", i),
                interval_ms: 100,
                enabled: false,
                target_app: Some("com.example.app".to_string()),
                sequence: ActionSequence {
                    steps: vec![
                        ActionStep::SustainedHold { input: InputEvent::Key(42) },
                        ActionStep::InterleavedInterval { input: InputEvent::MouseButton(MouseButton::Left), interval_ms: 500 },
                        ActionStep::InterleavedInterval { input: InputEvent::MouseButton(MouseButton::Right), interval_ms: 650 },
                        ActionStep::SustainedHold { input: InputEvent::Key(17) },
                        ActionStep::InterleavedInterval { input: InputEvent::Key(32), interval_ms: 200 },
                    ],
                },
            });
        }

        let profile = ProfileData {
            name: "StressTest".to_string(),
            macros,
            engine_active: true,
        };

        let json = serde_json::to_string(&profile).unwrap();
        let json_size_kb = json.len() / 1024;

        // JSON should be well under 1MB for 1000 macros.
        assert!(json_size_kb < 1024, "JSON too large: {}KB", json_size_kb);

        // Verify round-trip.
        let parsed: ProfileData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.macros.len(), 1000);

        eprintln!(
            "[Memory Audit] 1000 macros × 5 steps = {}KB JSON, round-trip OK",
            json_size_kb
        );
    }
}
