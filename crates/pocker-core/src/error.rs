//! Pocker error types.

use std::fmt;

/// Pocker's primary error type.
#[derive(Debug)]
pub enum PockerError {
    /// Plugin-related error (load/unload/registration)
    Plugin(String),
    /// Seam-related error (not found / already registered)
    Seam(String),
    /// Configuration error
    Config(String),
    /// Sandbox or runtime error
    Runtime(String),
    /// IO error
    Io(std::io::Error),
    /// Serialization error
    Serde(serde_json::Error),
    /// Generic error
    Other(anyhow::Error),
}

impl fmt::Display for PockerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PockerError::Plugin(msg) => write!(f, "plugin error: {msg}"),
            PockerError::Seam(msg) => write!(f, "seam error: {msg}"),
            PockerError::Config(msg) => write!(f, "config error: {msg}"),
            PockerError::Runtime(msg) => write!(f, "runtime error: {msg}"),
            PockerError::Io(e) => write!(f, "io error: {e}"),
            PockerError::Serde(e) => write!(f, "serde error: {e}"),
            PockerError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PockerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PockerError::Io(e) => Some(e),
            PockerError::Serde(e) => Some(e),
            PockerError::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PockerError {
    fn from(e: std::io::Error) -> Self {
        PockerError::Io(e)
    }
}

impl From<serde_json::Error> for PockerError {
    fn from(e: serde_json::Error) -> Self {
        PockerError::Serde(e)
    }
}

impl From<anyhow::Error> for PockerError {
    fn from(e: anyhow::Error) -> Self {
        PockerError::Other(e)
    }
}

impl From<serde_yaml::Error> for PockerError {
    fn from(e: serde_yaml::Error) -> Self {
        PockerError::Config(e.to_string())
    }
}

/// Result alias for Pocker operations.
pub type Result<T> = std::result::Result<T, PockerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_plugin() {
        let e = PockerError::Plugin("failed to load".into());
        assert_eq!(e.to_string(), "plugin error: failed to load");
    }

    #[test]
    fn error_display_seam() {
        let e = PockerError::Seam("ctx.llm not found".into());
        assert_eq!(e.to_string(), "seam error: ctx.llm not found");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let pocker_err: PockerError = io_err.into();
        assert!(matches!(pocker_err, PockerError::Io(_)));
    }

    #[test]
    fn error_from_serde_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let pocker_err: PockerError = json_err.into();
        assert!(matches!(pocker_err, PockerError::Serde(_)));
    }
}
