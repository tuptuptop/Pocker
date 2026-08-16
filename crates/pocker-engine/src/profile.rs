//! Profile manager — handles loading/saving/switching profiles.
//!
//! Profiles are stored in `~/.pocker/profiles/<name>/profile.yaml`.

use pocker_core::types::Profile;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("profile not found: {0}")]
    NotFound(String),
    #[error("profile already exists: {0}")]
    AlreadyExists(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("home directory not found")]
    NoHomeDir,
}

/// Manages profile files on disk.
pub struct ProfileManager {
    base_dir: PathBuf,
}

impl ProfileManager {
    /// Create a profile manager using the default ~/.pocker directory.
    ///
    /// # Errors
    /// Returns [`ProfileError::NoHomeDir`] if the user's home directory cannot
    /// be resolved.
    pub fn new() -> Result<Self, ProfileError> {
        let home = dirs::home_dir().ok_or(ProfileError::NoHomeDir)?;
        Ok(Self {
            base_dir: home.join(".pocker").join("profiles"),
        })
    }

    /// Create a profile manager with a custom base directory (for testing).
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Get the directory for a specific profile.
    #[must_use]
    pub fn profile_dir(&self, name: &str) -> PathBuf {
        self.base_dir.join(name)
    }

    /// Get the path to a profile's config file.
    #[must_use]
    pub fn profile_path(&self, name: &str) -> PathBuf {
        self.profile_dir(name).join("profile.yaml")
    }

    /// List all profile names.
    ///
    /// # Errors
    /// Returns [`ProfileError::Io`] if the profiles directory exists but cannot
    /// be read.
    pub fn list(&self) -> Result<Vec<String>, ProfileError> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }
        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    profiles.push(name.to_string());
                }
            }
        }
        profiles.sort();
        Ok(profiles)
    }

    /// Load a profile by name.
    ///
    /// # Errors
    /// Returns [`ProfileError::NotFound`] if the profile file is missing, or
    /// [`ProfileError::Io`]/[`ProfileError::Yaml`] if it cannot be read or
    /// parsed.
    pub fn load(&self, name: &str) -> Result<Profile, ProfileError> {
        let path = self.profile_path(name);
        if !path.exists() {
            return Err(ProfileError::NotFound(name.to_string()));
        }
        let contents = std::fs::read_to_string(&path)?;
        let profile: Profile = serde_yaml::from_str(&contents)?;
        Ok(profile)
    }

    /// Save a profile.
    ///
    /// # Errors
    /// Returns [`ProfileError::Io`] if the directory cannot be created or the
    /// file cannot be written, or [`ProfileError::Yaml`] if serialization
    /// fails.
    pub fn save(&self, profile: &Profile) -> Result<(), ProfileError> {
        let dir = self.profile_dir(&profile.name);
        std::fs::create_dir_all(&dir)?;
        let yaml = serde_yaml::to_string(profile)?;
        std::fs::write(self.profile_path(&profile.name), yaml)?;
        Ok(())
    }

    /// Create a new profile from a template.
    ///
    /// # Errors
    /// Returns [`ProfileError::AlreadyExists`] if a profile with the same name
    /// already exists, or propagates the error from [`ProfileManager::save`].
    pub fn create(
        &self,
        name: &str,
        description: &str,
        bundles: Vec<String>,
    ) -> Result<Profile, ProfileError> {
        if self.profile_path(name).exists() {
            return Err(ProfileError::AlreadyExists(name.to_string()));
        }
        let profile = Profile {
            name: name.to_string(),
            description: description.to_string(),
            bundles,
            plugins: Vec::new(),
            patch: serde_json::Value::Null,
        };
        self.save(&profile)?;
        Ok(profile)
    }

    /// Delete a profile.
    ///
    /// # Errors
    /// Returns [`ProfileError::NotFound`] if the profile directory does not
    /// exist, or [`ProfileError::Io`] if the directory cannot be removed.
    pub fn delete(&self, name: &str) -> Result<(), ProfileError> {
        let dir = self.profile_dir(name);
        if !dir.exists() {
            return Err(ProfileError::NotFound(name.to_string()));
        }
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// Check if a profile exists.
    #[must_use]
    pub fn exists(&self, name: &str) -> bool {
        self.profile_path(name).exists()
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            base_dir: PathBuf::from(".pocker/profiles"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn profile_create_and_load() {
        let tmp = tempdir().unwrap();
        let pm = ProfileManager::with_base_dir(tmp.path());

        let profile = pm
            .create(
                "test",
                "Test profile",
                vec!["@pocker/core-bundle".to_string()],
            )
            .unwrap();

        assert_eq!(profile.name, "test");
        assert!(pm.exists("test"));

        let loaded = pm.load("test").unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.description, "Test profile");
        assert_eq!(loaded.bundles, vec!["@pocker/core-bundle"]);
    }

    #[test]
    fn profile_list() {
        let tmp = tempdir().unwrap();
        let pm = ProfileManager::with_base_dir(tmp.path());

        pm.create("alpha", "Alpha", vec![]).unwrap();
        pm.create("beta", "Beta", vec![]).unwrap();

        let list = pm.list().unwrap();
        assert_eq!(list, vec!["alpha", "beta"]);
    }

    #[test]
    fn profile_delete() {
        let tmp = tempdir().unwrap();
        let pm = ProfileManager::with_base_dir(tmp.path());

        pm.create("temp", "Temp", vec![]).unwrap();
        assert!(pm.exists("temp"));

        pm.delete("temp").unwrap();
        assert!(!pm.exists("temp"));
    }

    #[test]
    fn profile_load_nonexistent() {
        let tmp = tempdir().unwrap();
        let pm = ProfileManager::with_base_dir(tmp.path());

        let result = pm.load("nonexistent");
        assert!(matches!(result, Err(ProfileError::NotFound(_))));
    }

    #[test]
    fn profile_create_duplicate() {
        let tmp = tempdir().unwrap();
        let pm = ProfileManager::with_base_dir(tmp.path());

        pm.create("dup", "First", vec![]).unwrap();
        let result = pm.create("dup", "Second", vec![]);
        assert!(matches!(result, Err(ProfileError::AlreadyExists(_))));
    }
}
