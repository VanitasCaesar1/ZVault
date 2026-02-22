//! Cloud repository — `PostgreSQL` queries for all cloud entities.
//!
//! Every function takes a `&PgPool` and returns `Result<T, CloudError>`.
//! Queries use parameterized statements (sqlx) — no SQL injection risk.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::CloudError;
use super::models::{
    AuditEntry, CloudUser, EncryptedSecret, Environment, OrgMember, Organization,
    Project, SecretKey, ServiceToken,
};

// ── Organizations ────────────────────────────────────────────────────

/// Create a new organization.
///
/// # Errors
///
/// Returns `CloudError::Conflict` if the slug is already taken.
pub async fn create_org(
    pool: &PgPool,
    name: &str,
    slug: &str,
    owner_id: Uuid,
    tier: &str,
    encryption_key: &[u8],
) -> Result<Organization, CloudError> {
    let org = sqlx::query_as::<_, Organization>(
        r"INSERT INTO organizations (name, slug, owner_id, tier, encryption_key)
          VALUES ($1, $2, $3, $4, $5)
          RETURNING *",
    )
    .bind(name)
    .bind(slug)
    .bind(owner_id)
    .bind(tier)
    .bind(encryption_key)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

/// Get an organization by ID.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the org does not exist.
pub async fn get_org(pool: &PgPool, org_id: Uuid) -> Result<Organization, CloudError> {
    sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
        .bind(org_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| CloudError::NotFound("organization not found".to_owned()))
}

/// List organizations for a user (as owner or member).
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_user_orgs(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Organization>, CloudError> {
    let orgs = sqlx::query_as::<_, Organization>(
        r"SELECT o.* FROM organizations o
          WHERE o.owner_id = $1
          UNION
          SELECT o.* FROM organizations o
          JOIN org_members m ON m.org_id = o.id
          WHERE m.user_id = $1 AND m.accepted_at IS NOT NULL
          ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(orgs)
}

// ── Members ──────────────────────────────────────────────────────────

/// Add a member to an organization.
///
/// # Errors
///
/// Returns `CloudError::Conflict` if the user is already a member.
pub async fn add_member(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
    email: &str,
    role: &str,
) -> Result<OrgMember, CloudError> {
    let member = sqlx::query_as::<_, OrgMember>(
        r"INSERT INTO org_members (org_id, user_id, email, role, accepted_at)
          VALUES ($1, $2, $3, $4, now())
          RETURNING *",
    )
    .bind(org_id)
    .bind(user_id)
    .bind(email)
    .bind(role)
    .fetch_one(pool)
    .await?;

    Ok(member)
}

/// List members of an organization.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_members(pool: &PgPool, org_id: Uuid) -> Result<Vec<OrgMember>, CloudError> {
    let members =
        sqlx::query_as::<_, OrgMember>("SELECT * FROM org_members WHERE org_id = $1 ORDER BY invited_at")
            .bind(org_id)
            .fetch_all(pool)
            .await?;

    Ok(members)
}

// ── Projects ─────────────────────────────────────────────────────────

/// Create a new project with default environments (development, staging, production).
///
/// # Errors
///
/// Returns `CloudError::Conflict` if the slug is already taken within the org.
pub async fn create_project(
    pool: &PgPool,
    org_id: Uuid,
    name: &str,
    slug: &str,
    description: &str,
) -> Result<Project, CloudError> {
    let mut tx = pool.begin().await.map_err(|e| CloudError::Internal(e.to_string()))?;

    let project = sqlx::query_as::<_, Project>(
        r"INSERT INTO projects (org_id, name, slug, description)
          VALUES ($1, $2, $3, $4)
          RETURNING *",
    )
    .bind(org_id)
    .bind(name)
    .bind(slug)
    .bind(description)
    .fetch_one(&mut *tx)
    .await?;

    // Create default environments.
    let defaults = [
        ("Development", "development", 0),
        ("Staging", "staging", 1),
        ("Production", "production", 2),
    ];

    for (env_name, env_slug, sort_order) in defaults {
        sqlx::query(
            r"INSERT INTO environments (project_id, name, slug, sort_order)
              VALUES ($1, $2, $3, $4)",
        )
        .bind(project.id)
        .bind(env_name)
        .bind(env_slug)
        .bind(sort_order)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await.map_err(|e| CloudError::Internal(e.to_string()))?;

    Ok(project)
}

/// List projects for an organization.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_projects(pool: &PgPool, org_id: Uuid) -> Result<Vec<Project>, CloudError> {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT * FROM projects WHERE org_id = $1 ORDER BY created_at DESC",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(projects)
}

/// Get a project by ID (with org ownership check).
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the project does not exist or belongs to another org.
pub async fn get_project(
    pool: &PgPool,
    project_id: Uuid,
    org_id: Uuid,
) -> Result<Project, CloudError> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1 AND org_id = $2")
        .bind(project_id)
        .bind(org_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| CloudError::NotFound("project not found".to_owned()))
}

// ── Environments ─────────────────────────────────────────────────────

/// List environments for a project.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_environments(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Vec<Environment>, CloudError> {
    let envs = sqlx::query_as::<_, Environment>(
        "SELECT * FROM environments WHERE project_id = $1 ORDER BY sort_order",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(envs)
}

/// Get an environment by project ID and slug.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the environment does not exist.
pub async fn get_environment_by_slug(
    pool: &PgPool,
    project_id: Uuid,
    slug: &str,
) -> Result<Environment, CloudError> {
    sqlx::query_as::<_, Environment>(
        "SELECT * FROM environments WHERE project_id = $1 AND slug = $2",
    )
    .bind(project_id)
    .bind(slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| CloudError::NotFound(format!("environment '{slug}' not found")))
}

/// Create a custom environment.
///
/// # Errors
///
/// Returns `CloudError::Conflict` if the slug already exists.
pub async fn create_environment(
    pool: &PgPool,
    project_id: Uuid,
    name: &str,
    slug: &str,
) -> Result<Environment, CloudError> {
    // Get next sort order.
    let max_order: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(sort_order) FROM environments WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let sort_order = max_order.unwrap_or(0).saturating_add(1);

    let env = sqlx::query_as::<_, Environment>(
        r"INSERT INTO environments (project_id, name, slug, sort_order)
          VALUES ($1, $2, $3, $4)
          RETURNING *",
    )
    .bind(project_id)
    .bind(name)
    .bind(slug)
    .bind(sort_order)
    .fetch_one(pool)
    .await?;

    Ok(env)
}

// ── Secrets ──────────────────────────────────────────────────────────

/// Upsert an encrypted secret (insert or update).
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn upsert_secret(
    pool: &PgPool,
    environment_id: Uuid,
    key: &str,
    encrypted_value: &[u8],
    nonce: &[u8],
    comment: &str,
    actor_id: Option<Uuid>,
) -> Result<EncryptedSecret, CloudError> {
    let secret = sqlx::query_as::<_, EncryptedSecret>(
        r"INSERT INTO cloud_secrets (environment_id, key, encrypted_value, nonce, comment, created_by, updated_by)
          VALUES ($1, $2, $3, $4, $5, $6, $6)
          ON CONFLICT (environment_id, key) DO UPDATE SET
            encrypted_value = EXCLUDED.encrypted_value,
            nonce = EXCLUDED.nonce,
            version = cloud_secrets.version + 1,
            comment = EXCLUDED.comment,
            updated_by = EXCLUDED.updated_by,
            updated_at = now()
          RETURNING *",
    )
    .bind(environment_id)
    .bind(key)
    .bind(encrypted_value)
    .bind(nonce)
    .bind(comment)
    .bind(actor_id)
    .fetch_one(pool)
    .await?;

    Ok(secret)
}

/// Get a single encrypted secret by key.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the secret does not exist.
pub async fn get_secret(
    pool: &PgPool,
    environment_id: Uuid,
    key: &str,
) -> Result<EncryptedSecret, CloudError> {
    sqlx::query_as::<_, EncryptedSecret>(
        "SELECT * FROM cloud_secrets WHERE environment_id = $1 AND key = $2",
    )
    .bind(environment_id)
    .bind(key)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| CloudError::NotFound(format!("secret '{key}' not found")))
}

/// List all encrypted secrets for an environment.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_secrets(
    pool: &PgPool,
    environment_id: Uuid,
) -> Result<Vec<EncryptedSecret>, CloudError> {
    let secrets = sqlx::query_as::<_, EncryptedSecret>(
        "SELECT * FROM cloud_secrets WHERE environment_id = $1 ORDER BY key",
    )
    .bind(environment_id)
    .fetch_all(pool)
    .await?;

    Ok(secrets)
}

/// List secret keys (no values) for an environment.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_secret_keys(
    pool: &PgPool,
    environment_id: Uuid,
) -> Result<Vec<SecretKey>, CloudError> {
    let keys = sqlx::query_as::<_, SecretKey>(
        r"SELECT key, version, comment, updated_at
          FROM cloud_secrets
          WHERE environment_id = $1
          ORDER BY key",
    )
    .bind(environment_id)
    .fetch_all(pool)
    .await?;

    Ok(keys)
}

/// Delete a secret by key.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the secret does not exist.
pub async fn delete_secret(
    pool: &PgPool,
    environment_id: Uuid,
    key: &str,
) -> Result<(), CloudError> {
    let result = sqlx::query(
        "DELETE FROM cloud_secrets WHERE environment_id = $1 AND key = $2",
    )
    .bind(environment_id)
    .bind(key)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(CloudError::NotFound(format!("secret '{key}' not found")));
    }

    Ok(())
}

// ── Service Tokens ───────────────────────────────────────────────────

/// Create a service token.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
#[allow(clippy::too_many_arguments)]
pub async fn create_service_token(
    pool: &PgPool,
    project_id: Uuid,
    environment_id: Option<Uuid>,
    name: &str,
    token_hash: &str,
    token_prefix: &str,
    permissions: &[String],
    expires_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
) -> Result<ServiceToken, CloudError> {
    let token = sqlx::query_as::<_, ServiceToken>(
        r"INSERT INTO service_tokens (project_id, environment_id, name, token_hash, token_prefix, permissions, expires_at, created_by)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
          RETURNING *",
    )
    .bind(project_id)
    .bind(environment_id)
    .bind(name)
    .bind(token_hash)
    .bind(token_prefix)
    .bind(permissions)
    .bind(expires_at)
    .bind(created_by)
    .fetch_one(pool)
    .await?;

    Ok(token)
}

/// Look up a service token by its SHA-256 hash.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the token does not exist or is revoked/expired.
pub async fn lookup_service_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<ServiceToken, CloudError> {
    sqlx::query_as::<_, ServiceToken>(
        r"SELECT * FROM service_tokens
          WHERE token_hash = $1
            AND revoked_at IS NULL
            AND (expires_at IS NULL OR expires_at > now())",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| CloudError::Unauthorized("invalid or expired service token".to_owned()))
}

/// Update `last_used_at` for a service token.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn touch_service_token(pool: &PgPool, token_id: Uuid) -> Result<(), CloudError> {
    sqlx::query("UPDATE service_tokens SET last_used_at = now() WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// List service tokens for a project.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_service_tokens(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Vec<ServiceToken>, CloudError> {
    let tokens = sqlx::query_as::<_, ServiceToken>(
        "SELECT * FROM service_tokens WHERE project_id = $1 ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(tokens)
}

/// Revoke a service token.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the token does not exist.
pub async fn revoke_service_token(
    pool: &PgPool,
    token_id: Uuid,
    project_id: Uuid,
) -> Result<(), CloudError> {
    let result = sqlx::query(
        "UPDATE service_tokens SET revoked_at = now() WHERE id = $1 AND project_id = $2",
    )
    .bind(token_id)
    .bind(project_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(CloudError::NotFound("service token not found".to_owned()));
    }

    Ok(())
}

// ── Users (Clerk-synced) ──────────────────────────────────────────────

/// Upsert a cloud user from Clerk JWT claims.
///
/// Creates the user on first API call, updates email/name on subsequent calls.
/// Clerk is the source of truth — this table caches user data for FK references.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn upsert_clerk_user(
    pool: &PgPool,
    clerk_id: &str,
    email: &str,
    name: &str,
) -> Result<CloudUser, CloudError> {
    let user = sqlx::query_as::<_, CloudUser>(
        r"INSERT INTO cloud_users (clerk_id, email, name)
          VALUES ($1, $2, $3)
          ON CONFLICT (clerk_id) DO UPDATE SET
            email = EXCLUDED.email,
            name = EXCLUDED.name,
            updated_at = now()
          RETURNING *",
    )
    .bind(clerk_id)
    .bind(email)
    .bind(name)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Get a user by Clerk ID.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the user does not exist.
pub async fn get_user_by_clerk_id(
    pool: &PgPool,
    clerk_id: &str,
) -> Result<CloudUser, CloudError> {
    sqlx::query_as::<_, CloudUser>("SELECT * FROM cloud_users WHERE clerk_id = $1")
        .bind(clerk_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| CloudError::NotFound("user not found".to_owned()))
}

/// Get a user by internal UUID.
///
/// # Errors
///
/// Returns `CloudError::NotFound` if the user does not exist.
pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<CloudUser, CloudError> {
    sqlx::query_as::<_, CloudUser>("SELECT * FROM cloud_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| CloudError::NotFound("user not found".to_owned()))
}

// ── Audit ────────────────────────────────────────────────────────────

/// Write an audit log entry.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
#[allow(clippy::too_many_arguments)]
pub async fn write_audit(
    pool: &PgPool,
    org_id: Uuid,
    project_id: Option<Uuid>,
    env_slug: Option<&str>,
    actor_id: Option<Uuid>,
    actor_type: &str,
    action: &str,
    resource: &str,
    detail: &serde_json::Value,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), CloudError> {
    sqlx::query(
        r"INSERT INTO cloud_audit_log (org_id, project_id, env_slug, actor_id, actor_type, action, resource, detail, ip_address, user_agent)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(org_id)
    .bind(project_id)
    .bind(env_slug)
    .bind(actor_id)
    .bind(actor_type)
    .bind(action)
    .bind(resource)
    .bind(detail)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await?;

    Ok(())
}

/// List audit entries for a project.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn list_audit(
    pool: &PgPool,
    project_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditEntry>, CloudError> {
    let entries = sqlx::query_as::<_, AuditEntry>(
        r"SELECT * FROM cloud_audit_log
          WHERE project_id = $1
          ORDER BY created_at DESC
          LIMIT $2 OFFSET $3",
    )
    .bind(project_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Check if a user has access to an organization (owner or accepted member).
///
/// # Errors
///
/// Returns `CloudError::Forbidden` if the user has no access.
pub async fn check_org_access(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<String, CloudError> {
    // Check if owner.
    let is_owner: Option<bool> = sqlx::query_scalar(
        "SELECT true FROM organizations WHERE id = $1 AND owner_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if is_owner.is_some() {
        return Ok("admin".to_owned());
    }

    // Check membership.
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM org_members WHERE org_id = $1 AND user_id = $2 AND accepted_at IS NOT NULL",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    role.ok_or_else(|| CloudError::Forbidden("no access to this organization".to_owned()))
}

/// Count environments for a project.
///
/// # Errors
///
/// Returns `CloudError::Internal` on database failure.
pub async fn count_environments(pool: &PgPool, project_id: Uuid) -> Result<i64, CloudError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM environments WHERE project_id = $1")
            .bind(project_id)
            .fetch_one(pool)
            .await?;

    Ok(count)
}
