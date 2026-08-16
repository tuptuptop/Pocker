//! Pocker Studio — Web GUI backend (Rust) serving the TypeScript frontend.

pub mod api;
pub mod server;

pub use server::run;
