//! Core library for `ZVault`.
//!
//! Contains the encryption barrier, cryptographic primitives, seal/unseal
//! logic, token store, policy engine, audit system, mount table, and lease
//! manager. This crate depends on `zvault-storage` for the storage backend
//! trait and knows nothing about specific secrets engines or auth methods.

pub mod approle;
pub mod audit;
pub mod audit_file;
pub mod barrier;
pub mod crypto;
pub mod database;
pub mod engine;
pub mod error;
pub mod lease;
pub mod mount;
pub mod pki;
pub mod policy;
pub mod seal;
pub mod token;
pub mod transit;
