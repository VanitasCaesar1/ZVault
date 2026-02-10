//! HTTP route handlers for `ZVault`.
//!
//! Routes are organized by subsystem:
//! - `sys`: System operations (init, seal, unseal, health)
//! - `auth`: Token authentication (create, lookup, renew, revoke)
//! - `policy`: Policy CRUD
//! - `mounts`: Engine mount management
//! - `leases`: Lease lifecycle
//! - `secrets`: Secret read/write through mounted engines
//! - `ui`: Landing page and web UI
//! - `dashboard`: Page content constants for the dashboard app

pub mod approle;
pub mod auth;
pub mod dashboard;
pub mod database;
pub mod docs;
pub mod leases;
pub mod mounts;
pub mod pki;
pub mod policy;
pub mod secrets;
pub mod sys;
pub mod transit;
pub mod ui;
