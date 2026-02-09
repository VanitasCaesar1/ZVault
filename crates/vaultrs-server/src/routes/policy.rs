//! Policy management routes: `/v1/sys/policies/*`
//!
//! CRUD operations for access control policies.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::middleware::AuthContext;
use crate::state::AppState;
use vaultrs_core::policy::{Capability, Policy, PolicyRule};

/// Build the `/v1/sys/policies` router.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_policies))
        .route("/{name}", get(get_policy))
        .route("/{name}", post(put_policy))
        .route("/{name}", delete(delete_policy))
}

// ── Request / Response types ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PolicyListResponse {
    pub policies: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PolicyResponse {
    pub name: String,
    pub rules: Vec<PolicyRuleResponse>,
}

#[derive(Debug, Serialize)]
pub struct PolicyRuleResponse {
    pub path: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PutPolicyRequest {
    pub rules: Vec<PutPolicyRule>,
}

#[derive(Debug, Deserialize)]
pub struct PutPolicyRule {
    pub path: String,
    pub capabilities: Vec<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// List all policy names.
async fn list_policies(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<PolicyListResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/policies", &Capability::List)
        .await?;

    let names = state.policy_store.list().await?;

    Ok(Json(PolicyListResponse { policies: names }))
}

/// Get a policy by name.
async fn get_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<PolicyResponse>, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/policies", &Capability::Read)
        .await?;

    let policy = state.policy_store.get(&name).await?;

    let rules = policy
        .rules
        .iter()
        .map(|r| PolicyRuleResponse {
            path: r.path.clone(),
            capabilities: r.capabilities.iter().map(|c| format!("{c:?}")).collect(),
        })
        .collect();

    Ok(Json(PolicyResponse {
        name: policy.name,
        rules,
    }))
}

/// Create or update a policy.
async fn put_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(body): Json<PutPolicyRequest>,
) -> Result<StatusCode, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/policies", &Capability::Create)
        .await?;

    let rules: Result<Vec<PolicyRule>, AppError> = body
        .rules
        .into_iter()
        .map(|r| {
            let capabilities: Result<Vec<Capability>, AppError> = r
                .capabilities
                .iter()
                .map(|c| parse_capability(c))
                .collect();
            Ok(PolicyRule {
                path: r.path,
                capabilities: capabilities?,
            })
        })
        .collect();

    let policy = Policy {
        name,
        rules: rules?,
    };

    state.policy_store.put(&policy).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Delete a policy.
async fn delete_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .policy_store
        .check(&auth.policies, "sys/policies", &Capability::Delete)
        .await?;

    state.policy_store.delete(&name).await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Parse a capability string into a [`Capability`] enum.
fn parse_capability(s: &str) -> Result<Capability, AppError> {
    match s.to_lowercase().as_str() {
        "read" => Ok(Capability::Read),
        "list" => Ok(Capability::List),
        "create" => Ok(Capability::Create),
        "update" => Ok(Capability::Update),
        "delete" => Ok(Capability::Delete),
        "sudo" => Ok(Capability::Sudo),
        "deny" => Ok(Capability::Deny),
        _ => Err(AppError::BadRequest(format!(
            "unknown capability: {s}"
        ))),
    }
}
