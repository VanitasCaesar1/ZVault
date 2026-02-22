//! Cloud authentication routes — user info endpoint.
//!
//! User authentication is handled by Clerk on the frontend. The backend
//! verifies Clerk JWTs from the `Authorization: Bearer` header via
//! middleware. These routes provide session info for the authenticated user.

use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::Serialize;
use sqlx::PgPool;

use crate::cloud::auth::CloudIdentity;
use crate::cloud::error::CloudError;

/// Response for the `/me` endpoint.
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user_id: String,
    pub email: String,
    pub auth_type: String,
}

/// Build the auth router.
///
/// The `/me` endpoint requires a valid Clerk JWT (via auth middleware).
/// Signup/login are handled entirely by Clerk on the frontend.
pub fn router() -> Router<PgPool> {
    Router::new().route("/me", get(me))
}

/// `GET /v1/cloud/auth/me` — get the current authenticated identity.
async fn me(
    Extension(identity): Extension<CloudIdentity>,
) -> Result<Json<MeResponse>, CloudError> {
    match identity {
        CloudIdentity::User {
            clerk_id, email, ..
        } => Ok(Json(MeResponse {
            user_id: clerk_id,
            email,
            auth_type: "clerk".to_owned(),
        })),
        CloudIdentity::ServiceToken { token_id, .. } => Ok(Json(MeResponse {
            user_id: token_id.to_string(),
            email: String::new(),
            auth_type: "service_token".to_owned(),
        })),
    }
}
