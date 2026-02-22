//! `ZVault` HTTP server.
//!
//! Wires together the core library, storage backend, and HTTP routes into a
//! running Axum server. Serves both the JSON API at `/v1/*` and the web UI
//! at `/`.

#[cfg(feature = "cloud")]
pub mod cloud;
pub mod config;
pub mod error;
pub mod hardening;
pub mod middleware;
pub mod routes;
pub mod state;
