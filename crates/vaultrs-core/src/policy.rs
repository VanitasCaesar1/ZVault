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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use vaultrs_storage::MemoryBackend;

    use super::*;
    use crate::barrier::Barrier;
    use crate::crypto::EncryptionKey;

    async fn make_policy_store() -> PolicyStore {
        let storage = Arc::new(MemoryBackend::new());
        let barrier = Arc::new(Barrier::new(storage));
        barrier.unseal(EncryptionKey::generate()).await;
        PolicyStore::new(barrier)
    }

    fn test_policy(name: &str, rules: Vec<PolicyRule>) -> Policy {
        Policy {
            name: name.to_owned(),
            rules,
        }
    }

    // ── CRUD ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn put_and_get_roundtrip() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "dev",
            vec![PolicyRule {
                path: "secret/data/dev/*".to_owned(),
                capabilities: vec![Capability::Read, Capability::List],
            }],
        );

        store.put(&policy).await.unwrap();
        let fetched = store.get("dev").await.unwrap();
        assert_eq!(fetched.name, "dev");
        assert_eq!(fetched.rules.len(), 1);
        assert_eq!(fetched.rules[0].capabilities.len(), 2);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_not_found() {
        let store = make_policy_store().await;
        let err = store.get("nonexistent").await.unwrap_err();
        assert!(matches!(err, PolicyError::NotFound { .. }));
    }

    #[tokio::test]
    async fn delete_removes_policy() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "temp",
            vec![PolicyRule {
                path: "secret/*".to_owned(),
                capabilities: vec![Capability::Read],
            }],
        );

        store.put(&policy).await.unwrap();
        store.delete("temp").await.unwrap();
        let err = store.get("temp").await.unwrap_err();
        assert!(matches!(err, PolicyError::NotFound { .. }));
    }

    #[tokio::test]
    async fn put_empty_rules_rejected() {
        let store = make_policy_store().await;
        let policy = test_policy("empty", vec![]);
        let err = store.put(&policy).await.unwrap_err();
        assert!(matches!(err, PolicyError::Invalid { .. }));
    }

    // ── Built-in policies ────────────────────────────────────────────

    #[tokio::test]
    async fn get_root_returns_builtin() {
        let store = make_policy_store().await;
        let root = store.get("root").await.unwrap();
        assert_eq!(root.name, "root");
        assert_eq!(root.rules.len(), 1);
        assert_eq!(root.rules[0].path, "**");
        assert!(root.rules[0].capabilities.contains(&Capability::Read));
        assert!(root.rules[0].capabilities.contains(&Capability::Sudo));
    }

    #[tokio::test]
    async fn get_default_returns_builtin() {
        let store = make_policy_store().await;
        let default = store.get("default").await.unwrap();
        assert_eq!(default.name, "default");
        assert_eq!(default.rules.len(), 2);
    }

    #[tokio::test]
    async fn cannot_modify_root_policy() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "root",
            vec![PolicyRule {
                path: "**".to_owned(),
                capabilities: vec![Capability::Read],
            }],
        );
        let err = store.put(&policy).await.unwrap_err();
        assert!(matches!(err, PolicyError::BuiltIn { .. }));
    }

    #[tokio::test]
    async fn cannot_modify_default_policy() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "default",
            vec![PolicyRule {
                path: "**".to_owned(),
                capabilities: vec![Capability::Read],
            }],
        );
        let err = store.put(&policy).await.unwrap_err();
        assert!(matches!(err, PolicyError::BuiltIn { .. }));
    }

    #[tokio::test]
    async fn cannot_delete_root_policy() {
        let store = make_policy_store().await;
        let err = store.delete("root").await.unwrap_err();
        assert!(matches!(err, PolicyError::BuiltIn { .. }));
    }

    #[tokio::test]
    async fn cannot_delete_default_policy() {
        let store = make_policy_store().await;
        let err = store.delete("default").await.unwrap_err();
        assert!(matches!(err, PolicyError::BuiltIn { .. }));
    }

    // ── list ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_includes_builtins() {
        let store = make_policy_store().await;
        let names = store.list().await.unwrap();
        assert!(names.contains(&"root".to_owned()));
        assert!(names.contains(&"default".to_owned()));
    }

    #[tokio::test]
    async fn list_includes_custom_policies() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "custom",
            vec![PolicyRule {
                path: "secret/*".to_owned(),
                capabilities: vec![Capability::Read],
            }],
        );
        store.put(&policy).await.unwrap();

        let names = store.list().await.unwrap();
        assert!(names.contains(&"custom".to_owned()));
        assert!(names.contains(&"root".to_owned()));
        assert!(names.contains(&"default".to_owned()));
    }

    // ── check (exact path match) ─────────────────────────────────────

    #[tokio::test]
    async fn check_exact_path_grants_access() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "exact",
            vec![PolicyRule {
                path: "secret/data/prod/db-password".to_owned(),
                capabilities: vec![Capability::Read],
            }],
        );
        store.put(&policy).await.unwrap();

        let result = store
            .check(
                &["exact".to_owned()],
                "secret/data/prod/db-password",
                &Capability::Read,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn check_exact_path_denies_wrong_capability() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "readonly",
            vec![PolicyRule {
                path: "secret/data/prod/db-password".to_owned(),
                capabilities: vec![Capability::Read],
            }],
        );
        store.put(&policy).await.unwrap();

        let result = store
            .check(
                &["readonly".to_owned()],
                "secret/data/prod/db-password",
                &Capability::Delete,
            )
            .await;
        assert!(matches!(result, Err(PolicyError::Denied { .. })));
    }

    // ── check (glob * — one level) ──────────────────────────────────

    #[tokio::test]
    async fn check_star_glob_matches_one_level() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "dev",
            vec![PolicyRule {
                path: "secret/data/dev/*".to_owned(),
                capabilities: vec![Capability::Read, Capability::Create],
            }],
        );
        store.put(&policy).await.unwrap();

        // Should match one level deep.
        let result = store
            .check(
                &["dev".to_owned()],
                "secret/data/dev/api-key",
                &Capability::Read,
            )
            .await;
        assert!(result.is_ok());
    }

    // ── check (glob ** — recursive) ─────────────────────────────────

    #[tokio::test]
    async fn check_double_star_glob_matches_recursively() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "admin",
            vec![PolicyRule {
                path: "secret/**".to_owned(),
                capabilities: vec![
                    Capability::Read,
                    Capability::Create,
                    Capability::Delete,
                ],
            }],
        );
        store.put(&policy).await.unwrap();

        // Should match deeply nested paths.
        let result = store
            .check(
                &["admin".to_owned()],
                "secret/data/prod/nested/deep/key",
                &Capability::Read,
            )
            .await;
        assert!(result.is_ok());
    }

    // ── deny overrides grant ─────────────────────────────────────────

    #[tokio::test]
    async fn deny_overrides_grant_in_same_policy() {
        let store = make_policy_store().await;
        let policy = test_policy(
            "mixed",
            vec![
                PolicyRule {
                    path: "secret/**".to_owned(),
                    capabilities: vec![Capability::Read],
                },
                PolicyRule {
                    path: "secret/data/prod/*".to_owned(),
                    capabilities: vec![Capability::Deny],
                },
            ],
        );
        store.put(&policy).await.unwrap();

        // The deny rule should override the grant.
        let result = store
            .check(
                &["mixed".to_owned()],
                "secret/data/prod/db-password",
                &Capability::Read,
            )
            .await;
        assert!(matches!(result, Err(PolicyError::Denied { .. })));
    }

    #[tokio::test]
    async fn deny_overrides_grant_across_policies() {
        let store = make_policy_store().await;

        let grant_policy = test_policy(
            "grant-all",
            vec![PolicyRule {
                path: "secret/**".to_owned(),
                capabilities: vec![Capability::Read, Capability::Create],
            }],
        );
        let deny_policy = test_policy(
            "deny-prod",
            vec![PolicyRule {
                path: "secret/data/prod/*".to_owned(),
                capabilities: vec![Capability::Deny],
            }],
        );
        store.put(&grant_policy).await.unwrap();
        store.put(&deny_policy).await.unwrap();

        let result = store
            .check(
                &["grant-all".to_owned(), "deny-prod".to_owned()],
                "secret/data/prod/api-key",
                &Capability::Read,
            )
            .await;
        assert!(matches!(result, Err(PolicyError::Denied { .. })));
    }

    // ── multiple policies with conflicting rules ─────────────────────

    #[tokio::test]
    async fn multiple_policies_union_capabilities() {
        let store = make_policy_store().await;

        let read_policy = test_policy(
            "reader",
            vec![PolicyRule {
                path: "secret/data/shared/*".to_owned(),
                capabilities: vec![Capability::Read],
            }],
        );
        let write_policy = test_policy(
            "writer",
            vec![PolicyRule {
                path: "secret/data/shared/*".to_owned(),
                capabilities: vec![Capability::Create],
            }],
        );
        store.put(&read_policy).await.unwrap();
        store.put(&write_policy).await.unwrap();

        // Read should work (from reader policy).
        let result = store
            .check(
                &["reader".to_owned(), "writer".to_owned()],
                "secret/data/shared/key",
                &Capability::Read,
            )
            .await;
        assert!(result.is_ok());

        // Create should work (from writer policy).
        let result = store
            .check(
                &["reader".to_owned(), "writer".to_owned()],
                "secret/data/shared/key",
                &Capability::Create,
            )
            .await;
        assert!(result.is_ok());

        // Delete should be denied (neither policy grants it).
        let result = store
            .check(
                &["reader".to_owned(), "writer".to_owned()],
                "secret/data/shared/key",
                &Capability::Delete,
            )
            .await;
        assert!(matches!(result, Err(PolicyError::Denied { .. })));
    }

    // ── root policy grants everything ────────────────────────────────

    #[tokio::test]
    async fn root_policy_grants_all_capabilities() {
        let store = make_policy_store().await;

        for cap in &[
            Capability::Read,
            Capability::List,
            Capability::Create,
            Capability::Update,
            Capability::Delete,
            Capability::Sudo,
        ] {
            let result = store
                .check(
                    &["root".to_owned()],
                    "any/arbitrary/path/here",
                    cap,
                )
                .await;
            assert!(result.is_ok(), "root should grant {cap:?}");
        }
    }

    // ── nonexistent policy is skipped ────────────────────────────────

    #[tokio::test]
    async fn nonexistent_policy_name_is_skipped() {
        let store = make_policy_store().await;

        // Only "ghost" policy referenced, which doesn't exist — should deny.
        let result = store
            .check(
                &["ghost".to_owned()],
                "secret/data/anything",
                &Capability::Read,
            )
            .await;
        assert!(matches!(result, Err(PolicyError::Denied { .. })));
    }

    // ── no policies means denied ─────────────────────────────────────

    #[tokio::test]
    async fn empty_policy_list_denies_access() {
        let store = make_policy_store().await;

        let result = store
            .check(&[], "secret/data/anything", &Capability::Read)
            .await;
        assert!(matches!(result, Err(PolicyError::Denied { .. })));
    }
}
