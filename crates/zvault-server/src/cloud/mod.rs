//! `ZVault` Cloud — multi-tenant secrets platform.
//!
//! This module implements the cloud API layer that transforms `ZVault` from a
//! local dev tool into a full secrets platform. Organizations manage secrets
//! per project and environment, with service tokens for CI/CD and production.
//!
//! # Authentication
//!
//! User authentication is handled by **Clerk** on the frontend. The backend
//! verifies Clerk JWTs and extracts user identity from claims. Service tokens
//! (`zvt_` prefix) are used by CI/CD pipelines and production runtimes.
//!
//! Billing and tier management are handled by **Clerk Billing** — the backend
//! reads the org's tier from the database (synced from Clerk webhooks or
//! checked via Clerk's API) to enforce feature gates like environment limits.
//!
//! # Architecture
//!
//! ```text
//! Cloud API (/v1/cloud/*)
//!   ├── auth/me (Clerk JWT → user info)
//!   ├── orgs (organization CRUD + members)
//!   ├── projects (project CRUD + environments)
//!   ├── secrets (per-environment secret CRUD, AES-256-GCM encrypted)
//!   └── tokens (service token management)
//! ```
//!
//! All secret values are encrypted with per-org AES-256-GCM keys before
//! storage. Nonces are generated fresh for every write via `OsRng`.

pub mod auth;
pub mod error;
pub mod models;
pub mod repository;
pub mod routes;
