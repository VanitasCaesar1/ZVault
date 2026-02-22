//! Cloud data models.
//!
//! Domain types for organizations, projects, environments, secrets,
//! service tokens, users, and sessions. All IDs are UUIDs. Secret values
//! are always encrypted — the `EncryptedSecret` type holds ciphertext + nonce.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Organizations ────────────────────────────────────────────────────

/// Pricing tier for an organization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum Tier {
    Free,
    Pro,
    Team,
    Business,
    Enterprise,
}

impl Tier {
    /// Maximum environments per project for this tier.
    #[must_use]
    pub const fn max_environments(&self) -> u32 {
        match self {
            Self::Free => 1,
            Self::Pro => 3,
            Self::Team => 5,
            Self::Business => 15,
            Self::Enterprise => u32::MAX,
        }
    }

    /// Maximum API requests per month for this tier.
    #[must_use]
    pub const fn max_api_requests_per_month(&self) -> u64 {
        match self {
            Self::Free => 0,
            Self::Pro => 50_000,
            Self::Team => 500_000,
            Self::Business => 5_000_000,
            Self::Enterprise => u64::MAX,
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Free => write!(f, "free"),
            Self::Pro => write!(f, "pro"),
            Self::Team => write!(f, "team"),
            Self::Business => write!(f, "business"),
            Self::Enterprise => write!(f, "enterprise"),
        }
    }
}

impl std::str::FromStr for Tier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "free" => Ok(Self::Free),
            "pro" => Ok(Self::Pro),
            "team" => Ok(Self::Team),
            "business" => Ok(Self::Business),
            "enterprise" => Ok(Self::Enterprise),
            other => Err(format!("unknown tier: {other}")),
        }
    }
}

/// An organization (tenant).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub owner_id: Uuid,
    pub tier: String,
    #[serde(skip)]
    pub encryption_key: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization member role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Admin,
    Developer,
    Viewer,
}

impl std::fmt::Display for MemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Viewer => write!(f, "viewer"),
        }
    }
}

/// An organization member.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OrgMember {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

// ── Projects ─────────────────────────────────────────────────────────

/// A project within an organization.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Environments ─────────────────────────────────────────────────────

/// An environment within a project (e.g., development, staging, production).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Environment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub slug: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

// ── Secrets ──────────────────────────────────────────────────────────

/// An encrypted secret stored in the cloud.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EncryptedSecret {
    pub id: Uuid,
    pub environment_id: Uuid,
    pub key: String,
    pub encrypted_value: Vec<u8>,
    pub nonce: Vec<u8>,
    pub version: i32,
    pub comment: String,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A decrypted secret (only exists in memory, never serialized with value to logs).
#[derive(Debug, Clone, Serialize)]
pub struct SecretEntry {
    pub key: String,
    pub value: String,
    pub version: i32,
    pub comment: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Secret key listing (no values).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SecretKey {
    pub key: String,
    pub version: i32,
    pub comment: String,
    pub updated_at: DateTime<Utc>,
}

// ── Service Tokens ───────────────────────────────────────────────────

/// A service token scoped to a project (optionally to an environment).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ServiceToken {
    pub id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Option<Uuid>,
    pub name: String,
    #[serde(skip)]
    pub token_hash: String,
    pub token_prefix: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

// ── Users & Sessions ─────────────────────────────────────────────────

/// A cloud user (synced from Clerk).
///
/// Users are created/updated on first API call via Clerk JWT claims.
/// Clerk is the source of truth for identity — this table caches user
/// data for foreign key references and audit logging.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CloudUser {
    pub id: Uuid,
    /// Clerk user ID (e.g., `user_2abc...`).
    pub clerk_id: String,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public user info (safe to serialize).
#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub clerk_id: String,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl From<CloudUser> for UserInfo {
    fn from(u: CloudUser) -> Self {
        Self {
            id: u.id,
            clerk_id: u.clerk_id,
            email: u.email,
            name: u.name,
            created_at: u.created_at,
        }
    }
}

// ── Audit ────────────────────────────────────────────────────────────

/// A cloud audit log entry.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AuditEntry {
    pub id: Uuid,
    pub org_id: Uuid,
    pub project_id: Option<Uuid>,
    pub env_slug: Option<String>,
    pub actor_id: Option<Uuid>,
    pub actor_type: String,
    pub action: String,
    pub resource: String,
    pub detail: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}
