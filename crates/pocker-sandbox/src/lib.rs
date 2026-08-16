//! Pocker Sandbox — cross-platform process isolation.
//!
//! Provides sandboxed execution for plugin code and tools.
//! Uses OS-native isolation mechanisms:
//! - Linux: namespaces + cgroups + seccomp
//! - Windows: Job Objects + `AppContainer`
//! - macOS: seatbelt (sandbox-exec)

pub mod process;

pub use process::Sandbox;
