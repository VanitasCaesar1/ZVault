//! Cloud API route handlers.
//!
//! All routes are nested under `/v1/cloud/` and require cloud authentication
//! (Clerk JWT or service token via `Authorization: Bearer` header).
//!
//! Auth is handled by Clerk on the frontend — the backend verifies Clerk JWTs
//! and extracts user identity from claims. Service tokens (`zvt_` prefix) are
//! used by CI/CD pipelines and production runtimes.

pub mod audit;
pub mod auth_routes;
pub mod orgs;
pub mod projects;
pub mod secrets;
pub mod tokens;

use axum::middleware as axum_mw;
use axum::Router;
use sqlx::PgPool;

use super::auth::cloud_auth_middleware;

/// Build the complete cloud API router.
///
/// All routes require authentication (Clerk JWT or service token).
/// Auth routes (`/me`) also require a valid token.
///
/// Returns a router with its own `PgPool` state fully applied, so it can
/// be merged into any parent router regardless of the parent's state type.
pub fn cloud_router(pool: PgPool) -> Router {
    let authenticated = Router::new()
        .merge(auth_routes::router())
        .merge(orgs::router())
        .merge(projects::router())
        .merge(secrets::router())
        .merge(tokens::router())
        .merge(audit::router())
        .route_layer(axum_mw::from_fn_with_state(
            pool.clone(),
            cloud_auth_middleware,
        ))
        .with_state(pool);

    Router::new().nest("/v1/cloud", authenticated)
}
