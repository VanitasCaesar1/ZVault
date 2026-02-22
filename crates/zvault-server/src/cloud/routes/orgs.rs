//! Organization management routes.
//!
//! Create, list, and inspect organizations. Invite and list members.
//! All routes require cloud authentication.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::cloud::auth::CloudIdentity;
use crate::cloud::error::CloudError;
use crate::cloud::models::{OrgMember, Organization};
use crate::cloud::repository;

/// Request body for creating an organization.
#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub slug: String,
}

/// Request body for inviting a member.
#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    #[serde(default)]
    pub user_id: Option<Uuid>,
    pub email: String,
    pub role: String,
}

/// Response for organization listing.
#[derive(Debug, Serialize)]
pub struct OrgListResponse {
    pub organizations: Vec<Organization>,
}

/// Response for member listing.
#[derive(Debug, Serialize)]
pub struct MemberListResponse {
    pub members: Vec<OrgMember>,
}

/// Build the organizations router.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/orgs", post(create_org).get(list_orgs))
        .route("/orgs/{org_id}", get(get_org))
        .route("/orgs/{org_id}/members", post(invite_member).get(list_members))
}

/// Generate a per-org AES-256-GCM encryption key.
///
/// Returns 32 bytes of randomness from the OS CSPRNG.
fn generate_org_encryption_key() -> Vec<u8> {
    use aes_gcm::aead::OsRng;
    use aes_gcm::aead::rand_core::RngCore;

    let mut key = vec![0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

/// `POST /v1/cloud/orgs` — create a new organization.
async fn create_org(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Json(body): Json<CreateOrgRequest>,
) -> Result<Json<Organization>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot create organizations".to_owned(),
        ));
    };

    if body.name.is_empty() {
        return Err(CloudError::BadRequest("name is required".to_owned()));
    }
    if body.slug.is_empty() || body.slug.len() > 64 {
        return Err(CloudError::BadRequest(
            "slug must be 1-64 characters".to_owned(),
        ));
    }

    let encryption_key = generate_org_encryption_key();

    let org = repository::create_org(
        &pool,
        &body.name,
        &body.slug,
        user_id,
        "free",
        &encryption_key,
    )
    .await?;

    Ok(Json(org))
}

/// `GET /v1/cloud/orgs` — list organizations for the current user.
async fn list_orgs(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
) -> Result<Json<OrgListResponse>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot list organizations".to_owned(),
        ));
    };

    let organizations = repository::list_user_orgs(&pool, user_id).await?;
    Ok(Json(OrgListResponse { organizations }))
}

/// `GET /v1/cloud/orgs/{org_id}` — get organization details.
async fn get_org(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<Organization>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot access organization details".to_owned(),
        ));
    };

    repository::check_org_access(&pool, org_id, user_id).await?;
    let org = repository::get_org(&pool, org_id).await?;

    Ok(Json(org))
}

/// `POST /v1/cloud/orgs/{org_id}/members` — invite a member.
async fn invite_member(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path(org_id): Path<Uuid>,
    Json(body): Json<InviteMemberRequest>,
) -> Result<Json<OrgMember>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot invite members".to_owned(),
        ));
    };

    // Only admins/owners can invite.
    let role = repository::check_org_access(&pool, org_id, user_id).await?;
    if role != "admin" {
        return Err(CloudError::Forbidden(
            "only admins can invite members".to_owned(),
        ));
    }

    if body.email.is_empty() || !body.email.contains('@') {
        return Err(CloudError::BadRequest("invalid email".to_owned()));
    }

    let valid_roles = ["admin", "developer", "viewer"];
    if !valid_roles.contains(&body.role.as_str()) {
        return Err(CloudError::BadRequest(format!(
            "role must be one of: {}",
            valid_roles.join(", ")
        )));
    }

    let member =
        repository::add_member(&pool, org_id, body.user_id.unwrap_or_else(Uuid::new_v4), &body.email, &body.role).await?;

    Ok(Json(member))
}

/// `GET /v1/cloud/orgs/{org_id}/members` — list organization members.
async fn list_members(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<MemberListResponse>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot list members".to_owned(),
        ));
    };

    repository::check_org_access(&pool, org_id, user_id).await?;
    let members = repository::list_members(&pool, org_id).await?;

    Ok(Json(MemberListResponse { members }))
}
