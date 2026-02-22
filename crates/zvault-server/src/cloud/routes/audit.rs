//! Cloud audit log routes.
//!
//! Read-only access to the cloud audit log for a project.
//! All routes require cloud authentication (Clerk JWT).

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::cloud::auth::CloudIdentity;
use crate::cloud::error::CloudError;
use crate::cloud::models::AuditEntry;
use crate::cloud::repository;

/// Query parameters for audit listing.
#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// Response for audit listing.
#[derive(Debug, Serialize)]
pub struct AuditListResponse {
    pub entries: Vec<AuditEntry>,
}

/// Build the audit router.
pub fn router() -> Router<PgPool> {
    Router::new().route(
        "/orgs/{org_id}/projects/{project_id}/audit",
        get(list_audit),
    )
}

/// `GET /v1/cloud/orgs/{org_id}/projects/{project_id}/audit`
///
/// List audit log entries for a project.
async fn list_audit(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEntry>>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot access audit logs".to_owned(),
        ));
    };

    repository::check_org_access(&pool, org_id, user_id).await?;
    repository::get_project(&pool, project_id, org_id).await?;

    let entries = repository::list_audit(&pool, project_id, query.limit, query.offset).await?;

    Ok(Json(entries))
}
