//! Policy engine for `ZVault`.
//!
//! Policies are JSON documents that define path-based access rules. Each rule
//! maps a path pattern to a set of capabilities (`read`, `list`, `create`,
//! `update`, `delete`, `sudo`, `deny`).
//!
//! Path matching supports:
//! - Exact: `secret/data/production/db-password`
//! - Glob: `secret/data/production/*` (one level)
//! - Recursive glob: `secret/data/**` (all descendants)
//!
//! `deny` always wins over other capabilities.
//!
//! Two built-in policies exist:
//! - `root`: grants all capabilities on all paths (attached to root token).
//! - `default`: grants basic self-management (token lookup/renew).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::barrier::Barrier;
use crate::error::PolicyError;

/// Storage prefix for policy documents.
const POLICY_PREFIX: &str = "sys/policies/";

/// A policy document containing access rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy name.
    pub name: String,
    /// Access rules.
    pub rules: Vec<PolicyRule>,
}

/// A single access rule within a policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Path pattern (supports `*` and `**` globs).
    pub path: String,
    /// Allowed capabilities on this path.
    pub capabilities: Vec<Capability>,
}

/// An access capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    /// Read secrets.
    Read,
    /// List keys under a prefix.
    List,
    /// Create new secrets.
    Create,
    /// Update existing secrets.
    Update,
    /// Delete secrets.
    Delete,
    /// Administrative operations.
    Sudo,
    /// Explicitly deny access (overrides all other capabilities).
    Deny,
}

/// Manages policy CRUD and evaluation.
pub struct PolicyStore {
    barrier: Arc<Barrier>,
}

impl PolicyStore {
    /// Create a new policy store backed by the given barrier.
    #[must_use]
    pub fn new(barrier: Arc<Barrier>) -> Self {
        Self { barrier }
    }

    /// Write or update a policy.
    ///
    /// # Errors
    ///
    /// - [`PolicyError::BuiltIn`] if trying to modify `root` or `default`.
    /// - [`PolicyError::Invalid`] if the policy has no rules.
    /// - [`PolicyError::Barrier`] if storage fails.
    pub async fn put(&self, policy: &Policy) -> Result<(), PolicyError> {
        if policy.name == "root" || policy.name == "default" {
            return Err(PolicyError::BuiltIn {
                name: policy.name.clone(),
            });
        }

        if policy.rules.is_empty() {
            return Err(PolicyError::Invalid {
                reason: "policy must have at least one rule".to_owned(),
            });
        }

        let bytes = serde_json::to_vec(policy).map_err(|e| PolicyError::Invalid {
            reason: format!("serialization failed: {e}"),
        })?;

        let key = format!("{POLICY_PREFIX}{}", policy.name);
        self.barrier.put(&key, &bytes).await?;

        info!(name = %policy.name, rules = policy.rules.len(), "policy written");

        Ok(())
    }

    /// Read a policy by name.
    ///
    /// Returns built-in policies for `root` and `default` without storage lookup.
    ///
    /// # Errors
    ///
    /// - [`PolicyError::NotFound`] if the policy doesn't exist.
    /// - [`PolicyError::Barrier`] if storage fails.
    pub async fn get(&self, name: &str) -> Result<Policy, PolicyError> {
        // Built-in policies.
        if name == "root" {
            return Ok(root_policy());
        }
        if name == "default" {
            return Ok(default_policy());
        }

        let key = format!("{POLICY_PREFIX}{name}");
        let data = self
            .barrier
            .get(&key)
            .await?
            .ok_or_else(|| PolicyError::NotFound {
                name: name.to_owned(),
            })?;

        serde_json::from_slice(&data).map_err(|e| PolicyError::Invalid {
            reason: format!("deserialization failed: {e}"),
        })
    }

    /// Delete a policy by name.
    ///
    /// # Errors
    ///
    /// - [`PolicyError::BuiltIn`] if trying to delete `root` or `default`.
    /// - [`PolicyError::Barrier`] if storage fails.
    pub async fn delete(&self, name: &str) -> Result<(), PolicyError> {
        if name == "root" || name == "default" {
            return Err(PolicyError::BuiltIn {
                name: name.to_owned(),
            });
        }

        let key = format!("{POLICY_PREFIX}{name}");
        self.barrier.delete(&key).await?;

        info!(name = %name, "policy deleted");

        Ok(())
    }

    /// List all policy names.
    ///
    /// Always includes `root` and `default`.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::Barrier`] if storage fails.
    pub async fn list(&self) -> Result<Vec<String>, PolicyError> {
        let keys = self.barrier.list(POLICY_PREFIX).await?;
        let mut names: Vec<String> = keys
            .iter()
            .filter_map(|k| k.strip_prefix(POLICY_PREFIX).map(String::from))
            .collect();

        // Always include built-ins.
        if !names.contains(&"root".to_owned()) {
            names.push("root".to_owned());
        }
        if !names.contains(&"default".to_owned()) {
            names.push("default".to_owned());
        }

        names.sort();
        Ok(names)
    }

    /// Check whether a set of policies grants a capability on a path.
    ///
    /// Loads each policy and evaluates rules. `deny` on any matching rule
    /// overrides all other grants.
    ///
    /// # Errors
    ///
    /// - [`PolicyError::Denied`] if no policy grants the capability.
    /// - [`PolicyError::Barrier`] if loading policies fails.
    pub async fn check(
        &self,
        policy_names: &[String],
        path: &str,
        capability: &Capability,
    ) -> Result<(), PolicyError> {
        let mut granted = false;

        for name in policy_names {
            let policy = match self.get(name).await {
                Ok(p) => p,
                Err(PolicyError::NotFound { .. }) => continue,
                Err(e) => return Err(e),
            };

            for rule in &policy.rules {
                if path_matches(&rule.path, path) {
                    // Deny always wins.
                    if rule.capabilities.contains(&Capability::Deny) {
                        return Err(PolicyError::Denied {
                            path: path.to_owned(),
                            capability: format!("{capability:?}"),
                        });
                    }
                    if rule.capabilities.contains(capability) {
                        granted = true;
                    }
                }
            }
        }

        if granted {
            Ok(())
        } else {
            Err(PolicyError::Denied {
                path: path.to_owned(),
                capability: format!("{capability:?}"),
            })
        }
    }
}

/// The built-in `root` policy — grants everything on all paths.
#[must_use]
pub fn root_policy() -> Policy {
    Policy {
        name: "root".to_owned(),
        rules: vec![PolicyRule {
            path: "**".to_owned(),
            capabilities: vec![
                Capability::Read,
                Capability::List,
                Capability::Create,
                Capability::Update,
                Capability::Delete,
                Capability::Sudo,
            ],
        }],
    }
}

/// The built-in `default` policy — basic self-management.
#[must_use]
pub fn default_policy() -> Policy {
    Policy {
        name: "default".to_owned(),
        rules: vec![
            PolicyRule {
                path: "auth/token/lookup-self".to_owned(),
                capabilities: vec![Capability::Read],
            },
            PolicyRule {
                path: "auth/token/renew-self".to_owned(),
                capabilities: vec![Capability::Update],
            },
        ],
    }
}

/// Match a path against a pattern supporting `*` (one segment) and `**` (recursive).
fn path_matches(pattern: &str, path: &str) -> bool {
    glob_match::glob_match(pattern, path)
}

impl std::fmt::Debug for PolicyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyStore").finish_non_exhaustive()
    }
}
