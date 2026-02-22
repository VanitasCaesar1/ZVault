//! Project and environment management routes.
//!
//! Create and list projects within an organization. Manage environments
//! per project with tier-based limits on environment count.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::cloud::auth::CloudIdentity;
use crate::cloud::error::CloudError;
use crate::cloud::models::{Environment, Project, Tier};
use crate::cloud::repository;

/// Request body for creating a project.
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub description: String,
}

/// Request body for creating an environment.
#[derive(Debug, Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub slug: String,
}

/// Response for project listing.
#[derive(Debug, Serialize)]
pub struct ProjectListResponse {
    pub projects: Vec<Project>,
}

/// Response for environment listing.
#[derive(Debug, Serialize)]
pub struct EnvironmentListResponse {
    pub environments: Vec<Environment>,
}

/// Build the projects router.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route(
            "/orgs/{org_id}/projects",
            post(create_project).get(list_projects),
        )
        .route("/orgs/{org_id}/projects/{project_id}", get(get_project))
        .route(
            "/orgs/{org_id}/projects/{project_id}/environments",
            post(create_environment).get(list_environments),
        )
}

/// `POST /v1/cloud/orgs/{org_id}/projects` — create a new project.
async fn create_project(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path(org_id): Path<Uuid>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<Json<Project>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot create projects".to_owned(),
        ));
    };

    let role = repository::check_org_access(&pool, org_id, user_id).await?;
    if role == "viewer" {
        return Err(CloudError::Forbidden(
            "viewers cannot create projects".to_owned(),
        ));
    }

    if body.name.is_empty() {
        return Err(CloudError::BadRequest("name is required".to_owned()));
    }
    if body.slug.is_empty() || body.slug.len() > 64 {
        return Err(CloudError::BadRequest(
            "slug must be 1-64 characters".to_owned(),
        ));
    }

    let project =
        repository::create_project(&pool, org_id, &body.name, &body.slug, &body.description)
            .await?;

    Ok(Json(project))
}

/// `GET /v1/cloud/orgs/{org_id}/projects` — list projects.
async fn list_projects(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ProjectListResponse>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot list projects".to_owned(),
        ));
    };

    repository::check_org_access(&pool, org_id, user_id).await?;
    let projects = repository::list_projects(&pool, org_id).await?;

    Ok(Json(ProjectListResponse { projects }))
}

/// `GET /v1/cloud/orgs/{org_id}/projects/{project_id}` — get project details.
async fn get_project(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Project>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot access project details".to_owned(),
        ));
    };

    repository::check_org_access(&pool, org_id, user_id).await?;
    let project = repository::get_project(&pool, project_id, org_id).await?;

    Ok(Json(project))
}

/// `GET /v1/cloud/orgs/{org_id}/projects/{project_id}/environments` — list environments.
async fn list_environments(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<EnvironmentListResponse>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot list environments".to_owned(),
        ));
    };

    repository::check_org_access(&pool, org_id, user_id).await?;
    // Verify project belongs to org.
    repository::get_project(&pool, project_id, org_id).await?;

    let environments = repository::list_environments(&pool, project_id).await?;

    Ok(Json(EnvironmentListResponse { environments }))
}

/// `POST /v1/cloud/orgs/{org_id}/projects/{project_id}/environments` — create environment.
///
/// Enforces tier-based limits on the number of environments per project.
async fn create_environment(
    State(pool): State<PgPool>,
    Extension(identity): Extension<CloudIdentity>,
    Path((org_id, project_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> Result<Json<Environment>, CloudError> {
    let CloudIdentity::User { user_id, .. } = identity else {
        return Err(CloudError::Forbidden(
            "service tokens cannot create environments".to_owned(),
        ));
    };

    let role = repository::check_org_access(&pool, org_id, user_id).await?;
    if role == "viewer" {
        return Err(CloudError::Forbidden(
            "viewers cannot create environments".to_owned(),
        ));
    }

    // Verify project belongs to org.
    repository::get_project(&pool, project_id, org_id).await?;

    // Enforce tier limits.
    let org = repository::get_org(&pool, org_id).await?;
    let tier: Tier = org
        .tier
        .parse()
        .map_err(|e: String| CloudError::Internal(e))?;
    let current_count = repository::count_environments(&pool, project_id).await?;
    let max_envs = i64::from(tier.max_environments());

    if current_count >= max_envs {
        return Err(CloudError::LimitExceeded(format!(
            "{tier} tier allows max {} environments per project",
            tier.max_environments()
        )));
    }

    if body.name.is_empty() {
        return Err(CloudError::BadRequest("name is required".to_owned()));
    }
    if body.slug.is_empty() || body.slug.len() > 64 {
        return Err(CloudError::BadRequest(
            "slug must be 1-64 characters".to_owned(),
        ));
    }

    let env =
        repository::create_environment(&pool, project_id, &body.name, &body.slug).await?;

    Ok(Json(env))
}
