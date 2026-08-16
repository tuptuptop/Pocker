//! Plugin trait and metadata.
//!
//! Every part of Pocker is a plugin. Plugins mount into the shared context,
//! register their services on seams, and automatically unwind on unmount.

use crate::context::Ctx;
use crate::error::Result;
use crate::types::PluginType;
use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fmt::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Content-addressable identity of a plugin — the analog of a Docker layer
/// digest. Computed from the plugin's manifest plus its implementation hash,
/// so identical bytes always yield the same digest, everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginDigest(pub [u8; 32]);

impl PluginDigest {
    /// The zero digest — a sentinel used for non-plugin / test registrations
    /// that are not content-addressed.
    #[must_use]
    pub fn empty() -> Self {
        Self([0u8; 32])
    }

    /// Return the raw 32-byte digest.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex encoding (64 chars).
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            let _ = write!(s, "{:02x}", b);
        }
        s
    }

    /// Parse a 64-char lowercase hex string.
    ///
    /// # Errors
    /// Returns an error if the string is not exactly 64 hex characters.
    pub fn from_hex(s: &str) -> std::result::Result<Self, String> {
        if s.len() != 64 {
            return Err(format!("invalid digest length: expected 64, got {}", s.len()));
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let piece = std::str::from_utf8(chunk).map_err(|e| e.to_string())?;
            bytes[i] = u8::from_str_radix(piece, 16).map_err(|e| e.to_string())?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for PluginDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for PluginDigest {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for PluginDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        PluginDigest::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// Metadata describing a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique name (e.g. "@pocker/llm-openai")
    pub name: String,
    /// Semantic version
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Plugin type
    pub plugin_type: PluginType,
    /// Seams this plugin provides
    pub provides: Vec<String>,
    /// Seams this plugin requires (dependencies)
    pub requires: Vec<String>,
    /// Content hash of the plugin's implementation artifact (wasm/binary/source).
    /// Supplied by the build; enables full content-addressing. `None` means the
    /// digest covers the manifest only.
    #[serde(default)]
    pub code_hash: Option<[u8; 32]>,
}

impl PluginMetadata {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: String::new(),
            plugin_type: PluginType::Other,
            provides: Vec::new(),
            requires: Vec::new(),
            code_hash: None,
        }
    }

    /// Attach an implementation content hash, enabling full content-addressing.
    #[must_use]
    pub fn with_code_hash(mut self, hash: [u8; 32]) -> Self {
        self.code_hash = Some(hash);
        self
    }

    /// Compute the content-addressable digest of this plugin.
    ///
    /// The digest covers the full manifest (name, version, description, type,
    /// provides, requires) plus the optional `code_hash`. It is deterministic:
    /// identical content always yields the same digest, on every machine. This
    /// is what makes the plugin the minimal hashable unit — Pocker's "layer".
    #[must_use]
    pub fn digest(&self) -> PluginDigest {
        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());
        hasher.update([0x1f]);
        hasher.update(self.version.as_bytes());
        hasher.update([0x1f]);
        hasher.update(self.description.as_bytes());
        hasher.update([0x1f]);
        if let Ok(pt) = serde_json::to_string(&self.plugin_type) {
            hasher.update(pt.as_bytes());
        }
        hasher.update([0x1f]);
        let mut provides = self.provides.clone();
        provides.sort();
        for p in &provides {
            hasher.update(p.as_bytes());
            hasher.update([0x1f]);
        }
        let mut requires = self.requires.clone();
        requires.sort();
        for r in &requires {
            hasher.update(r.as_bytes());
            hasher.update([0x1f]);
        }
        if let Some(ch) = self.code_hash {
            hasher.update(ch);
        }
        PluginDigest(hasher.finalize().into())
    }
}

/// Runtime state of a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Created but not yet mounted
    Created,
    /// Currently mounting
    Mounting,
    /// Mounted and active
    Mounted,
    /// Currently unmounting
    Unmounting,
    /// Unmounted (can be remounted)
    Unmounted,
    /// Failed to mount
    Failed,
}

/// A plugin. Plugins contribute services, events, and reversible effects
/// to the shared context.
///
/// The `mount` method registers the plugin's services on seams.
/// The `unmount` method reverses those registrations (unwinds effects).
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Metadata about this plugin.
    fn metadata(&self) -> &PluginMetadata;

    /// Mount this plugin into the shared context.
    /// Register services, subscribe to events, etc.
    async fn mount(&self, ctx: &Arc<Ctx>) -> Result<()>;

    /// Unmount this plugin from the shared context.
    /// All registrations made in `mount` should be reversed.
    async fn unmount(&self, ctx: &Arc<Ctx>) -> Result<()>;
}

/// A handle to a loaded plugin, tracking its state.
pub struct PluginHandle {
    pub plugin: Arc<dyn Plugin>,
    pub state: Arc<AtomicBool>, // true = mounted
}

impl PluginHandle {
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self {
            plugin,
            state: Arc::new(AtomicBool::new(false)),
        }
    }

    #[must_use]
    pub fn is_mounted(&self) -> bool {
        self.state.load(Ordering::SeqCst)
    }

    pub fn set_mounted(&self, mounted: bool) {
        self.state.store(mounted, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_new() {
        let meta = PluginMetadata::new("test-plugin", "1.0.0");
        assert_eq!(meta.name, "test-plugin");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.plugin_type, PluginType::Other);
    }

    #[test]
    fn plugin_handle_state() {
        struct DummyPlugin {
            meta: PluginMetadata,
        }

        #[async_trait]
        impl Plugin for DummyPlugin {
            fn metadata(&self) -> &PluginMetadata {
                &self.meta
            }
            async fn mount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
                Ok(())
            }
            async fn unmount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
                Ok(())
            }
        }

        let plugin = Arc::new(DummyPlugin {
            meta: PluginMetadata::new("dummy", "0.1.0"),
        });
        let handle = PluginHandle::new(plugin);

        assert!(!handle.is_mounted());
        handle.set_mounted(true);
        assert!(handle.is_mounted());
    }

    #[test]
    fn metadata_digest_is_deterministic() {
        let a = PluginMetadata::new("p", "1.0.0");
        let b = PluginMetadata::new("p", "1.0.0");
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn metadata_digest_changes_with_version() {
        let a = PluginMetadata::new("p", "1.0.0");
        let b = PluginMetadata::new("p", "1.0.1");
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn metadata_digest_changes_with_code_hash() {
        let a = PluginMetadata::new("p", "1.0.0");
        let b = PluginMetadata::new("p", "1.0.0").with_code_hash([7u8; 32]);
        assert_ne!(a.digest(), b.digest());
        assert_eq!(
            b.digest(),
            PluginMetadata::new("p", "1.0.0").with_code_hash([7u8; 32]).digest()
        );
    }

    #[test]
    fn plugin_digest_hex_roundtrip() {
        let d = PluginMetadata::new("p", "1.0.0").digest();
        let hex = d.to_hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(PluginDigest::from_hex(&hex).unwrap(), d);
        assert!(PluginDigest::from_hex("zz").is_err());
        assert!(PluginDigest::from_hex(&"a".repeat(63)).is_err());
    }

    #[test]
    fn plugin_digest_empty_is_all_zeros() {
        assert_eq!(PluginDigest::empty().as_bytes(), &[0u8; 32]);
        assert_eq!(PluginDigest::empty().to_hex(), "0".repeat(64));
    }

    #[test]
    fn plugin_digest_json_roundtrip() {
        let d = PluginMetadata::new("p", "1.0.0").digest();
        let json = serde_json::to_string(&d).unwrap();
        let back: PluginDigest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }
}
