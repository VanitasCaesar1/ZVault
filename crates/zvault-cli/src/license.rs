//! License verification for `ZVault` Pro/Team/Enterprise features.
//!
//! License keys are Ed25519-signed JSON payloads encoded as base64. Verification
//! is performed locally using an embedded public key — no phone-home required.
//!
//! # License Key Format
//!
//! A license key is a base64-encoded JSON object concatenated with a `.` separator
//! and a base64-encoded Ed25519 signature:
//!
//! ```text
//! <base64(payload)>.<base64(signature)>
//! ```
//!
//! The payload contains:
//! ```json
//! {
//!   "tier": "pro",
//!   "email": "user@example.com",
//!   "issued_at": "2026-02-11T00:00:00Z",
//!   "expires_at": "2027-02-11T00:00:00Z",
//!   "license_id": "lic_abc123"
//! }
//! ```

use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

// ── Polar.sh configuration ───────────────────────────────────────────
//
// Organization ID from the Polar dashboard. Used for license key
// validation and activation against the Polar API.
const POLAR_ORG_ID: &str = "2eb1c165-a876-4932-baf7-6119c4c06816";
const POLAR_API_BASE: &str = "https://api.polar.sh/v1/customer-portal/license-keys";

// ── Embedded public key ──────────────────────────────────────────────
//
// This is the Ed25519 public key used to verify license signatures.
// The corresponding private key is held by the ZVault license server
// (Lemon Squeezy / Polar.sh webhook handler).
//
// To generate a new keypair for production:
//   use ed25519_dalek::SigningKey;
//   use rand::rngs::OsRng;
//   let sk = SigningKey::generate(&mut OsRng);
//   let pk = sk.verifying_key();
//   println!("public: {}", base64::encode(pk.as_bytes()));
//   println!("secret: {}", base64::encode(sk.to_bytes()));
//
// Replace this placeholder with the real public key before shipping.
const PUBLIC_KEY_B64: &str = "/3mEyrpmgX5NhAd9vLGaN7wI2JraX4Q2zrEQEUcor/M=";

// ── License tiers ────────────────────────────────────────────────────

/// Feature tier for a `ZVault` license.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Free tier — local vault, CLI, .env import.
    Free,
    /// Pro ($8/mo) — AI Mode, MCP server, zvault://, llms.txt.
    Pro,
    /// Team ($19/mo) — shared vault, OIDC, audit export, Slack alerts.
    Team,
    /// Enterprise ($49/mo) — HA, K8s operator, namespaces, SLA.
    Enterprise,
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Free => write!(f, "Free"),
            Self::Pro => write!(f, "Pro"),
            Self::Team => write!(f, "Team"),
            Self::Enterprise => write!(f, "Enterprise"),
        }
    }
}

// ── License payload ──────────────────────────────────────────────────

/// The signed payload inside a license key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// Feature tier.
    pub tier: Tier,
    /// Licensee email address.
    pub email: String,
    /// ISO 8601 timestamp when the license was issued.
    pub issued_at: String,
    /// ISO 8601 timestamp when the license expires.
    pub expires_at: String,
    /// Unique license identifier (e.g. `lic_abc123`).
    pub license_id: String,
}

/// A verified license with its decoded payload.
#[derive(Debug, Clone)]
pub struct License {
    pub payload: LicensePayload,
    /// The raw license key string (for storage).
    #[allow(dead_code)]
    pub raw_key: String,
}

// ── License file location ────────────────────────────────────────────

/// Returns the path to the license file.
///
/// Checks in order:
/// 1. `.zvault/license.key` in the current directory (project-local)
/// 2. `~/.zvault/license.key` (user-global)
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
pub fn license_file_path() -> Result<PathBuf> {
    // Project-local takes precedence.
    let local = PathBuf::from(".zvault/license.key");
    if local.exists() {
        return Ok(local);
    }

    // Fall back to user-global.
    let home = home_dir()?;
    Ok(home.join(".zvault").join("license.key"))
}

/// Returns the user's home directory.
fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .context("cannot determine home directory (HOME / USERPROFILE not set)")
}

// ── Core verification ────────────────────────────────────────────────

/// Decode and verify a license key string.
///
/// The key format is `<base64(payload)>.<base64(signature)>`.
///
/// # Errors
///
/// Returns an error if:
/// - The key format is invalid (missing `.` separator)
/// - Base64 decoding fails
/// - The Ed25519 signature is invalid
/// - The payload JSON is malformed
/// - The license has expired
/// - The payload fields fail sanity checks
pub fn verify_license_key(key: &str) -> Result<License> {
    let key = key.trim();

    // Reject obviously bogus keys early.
    if key.len() < 16 || key.len() > 4096 {
        bail!("invalid license key length");
    }

    let (payload_b64, sig_b64) = key
        .rsplit_once('.')
        .ok_or_else(|| anyhow::anyhow!("invalid license key format (missing separator)"))?;

    // Decode payload.
    let payload_bytes = BASE64
        .decode(payload_b64)
        .context("invalid license key (payload decode failed)")?;

    // Decode signature.
    let sig_bytes = BASE64
        .decode(sig_b64)
        .context("invalid license key (signature decode failed)")?;

    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| anyhow::anyhow!("invalid signature format: {e}"))?;

    // Load the embedded public key.
    let pk_bytes = BASE64
        .decode(PUBLIC_KEY_B64)
        .context("embedded public key is invalid (this is a build error)")?;

    let pk_array: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("embedded public key has wrong length (expected 32 bytes)"))?;

    let verifying_key = VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| anyhow::anyhow!("invalid embedded public key: {e}"))?;

    // Verify signature over the raw base64 payload string (not decoded bytes).
    // This matches how the license server signs: sign(base64(payload)).
    verifying_key
        .verify(payload_b64.as_bytes(), &signature)
        .map_err(|_| {
            anyhow::anyhow!("license signature verification failed — key is invalid or tampered")
        })?;

    // Parse payload JSON.
    let payload: LicensePayload =
        serde_json::from_slice(&payload_bytes).context("license payload is malformed JSON")?;

    // ── Payload sanity checks ────────────────────────────────────
    // These prevent crafted-but-signed payloads from causing issues
    // and catch corrupted data early.

    validate_payload(&payload)?;

    // Check expiration (simple string comparison works for ISO 8601).
    let now = now_iso8601();
    if payload.expires_at < now {
        bail!(
            "license expired on {} (current time: {now})",
            payload.expires_at
        );
    }

    // Reject licenses issued in the future (clock skew tolerance: 1 day).
    // This catches forged licenses with absurd issued_at dates.
    if payload.issued_at > now {
        // Allow up to 24h of clock skew.
        let issued_date = &payload.issued_at[..10]; // YYYY-MM-DD
        let now_date = &now[..10];
        if issued_date > now_date {
            bail!(
                "license issued_at ({}) is in the future — clock skew or forged key",
                payload.issued_at
            );
        }
    }

    Ok(License {
        payload,
        raw_key: key.to_owned(),
    })
}

/// Validate payload fields for sanity.
///
/// # Errors
///
/// Returns an error if any field is out of expected bounds.
fn validate_payload(payload: &LicensePayload) -> Result<()> {
    // Email: basic length check (not full RFC 5322, just sanity).
    if payload.email.is_empty() || payload.email.len() > 320 {
        bail!("license email is invalid (empty or too long)");
    }
    if !payload.email.contains('@') {
        bail!("license email is invalid (missing @)");
    }

    // License ID: must be non-empty, reasonable length.
    if payload.license_id.is_empty() || payload.license_id.len() > 128 {
        bail!("license_id is invalid");
    }

    // Timestamps: must look like ISO 8601 (YYYY-MM-DDTHH:MM:SSZ).
    if !is_iso8601_like(&payload.issued_at) {
        bail!("issued_at is not a valid ISO 8601 timestamp");
    }
    if !is_iso8601_like(&payload.expires_at) {
        bail!("expires_at is not a valid ISO 8601 timestamp");
    }

    // Expiry must be after issuance.
    if payload.expires_at <= payload.issued_at {
        bail!("expires_at must be after issued_at");
    }

    // Max license duration: 5 years. Prevents "expires 9999-12-31" abuse.
    // Compare year portion only for simplicity.
    if let (Ok(issued_year), Ok(expires_year)) = (
        payload.issued_at[..4].parse::<u32>(),
        payload.expires_at[..4].parse::<u32>(),
    ) && expires_year.saturating_sub(issued_year) > 5
    {
        bail!("license duration exceeds maximum (5 years)");
    }

    Ok(())
}

/// Quick check that a string looks like `YYYY-MM-DDTHH:MM:SSZ`.
fn is_iso8601_like(s: &str) -> bool {
    // Must be exactly 20 chars: 2026-02-11T00:00:00Z
    if s.len() != 20 {
        return false;
    }
    let b = s.as_bytes();
    // Check separators at known positions.
    b[4] == b'-' && b[7] == b'-' && b[10] == b'T' && b[13] == b':' && b[16] == b':' && b[19] == b'Z'
}

// Time constants for ISO 8601 conversion.
const SECS_PER_MIN: u64 = 60;
const SECS_PER_HOUR: u64 = 3600;
const SECS_PER_DAY: u64 = 86400;

/// Returns the current UTC time as an ISO 8601 string.
///
/// Format: `YYYY-MM-DDTHH:MM:SSZ`
///
/// Uses `std::time::SystemTime` to avoid adding a datetime dependency.
fn now_iso8601() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    let days = secs / SECS_PER_DAY;
    let time_of_day = secs % SECS_PER_DAY;
    let hour = time_of_day / SECS_PER_HOUR;
    let minute = (time_of_day % SECS_PER_HOUR) / SECS_PER_MIN;
    let second = time_of_day % SECS_PER_MIN;

    // Civil date from days since epoch (algorithm from Howard Hinnant).
    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Convert days since Unix epoch to (year, month, day).
///
/// Algorithm by Howard Hinnant.
/// See: <https://howardhinnant.github.io/date_algorithms.html#civil_from_days>
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days.saturating_add(719_468);
    let era = z / 146_097;
    let doe = z.saturating_sub(era.saturating_mul(146_097)); // day of era
    let yoe = (doe
        .saturating_sub(doe / 1460)
        .saturating_add(doe / 36524)
        .saturating_sub(doe / 146_096))
        / 365;
    let y = yoe.saturating_add(era.saturating_mul(400));
    let doy = doe.saturating_sub(
        yoe.saturating_mul(365)
            .saturating_add(yoe / 4)
            .saturating_sub(yoe / 100),
    );
    let mp = (doy.saturating_mul(5).saturating_add(2)) / 153;
    let d = doy
        .saturating_sub(mp.saturating_mul(153).saturating_add(2) / 5)
        .saturating_add(1);
    let m = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    };
    let y = if m <= 2 { y.saturating_add(1) } else { y };

    (y, m, d)
}

// ── Polar.sh license key validation ──────────────────────────────────

/// Response from Polar's license key validation endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PolarValidateResponse {
    #[serde(default)]
    id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    customer_id: Option<String>,
    #[serde(default)]
    benefit_id: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
}

/// Response from Polar's license key activation endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PolarActivateResponse {
    #[serde(default)]
    id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    expires_at: Option<String>,
}

/// Cached Polar license stored locally after successful validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolarLicenseCache {
    /// The raw Polar license key.
    pub key: String,
    /// Resolved tier based on the Polar product/benefit.
    pub tier: Tier,
    /// When the license was last validated against Polar API.
    pub validated_at: String,
    /// When the license expires (from Polar), if set.
    pub expires_at: Option<String>,
    /// Polar license key ID.
    pub license_id: String,
}

/// Detect whether a key is a Polar license key vs an Ed25519-signed key.
///
/// Polar keys don't contain a `.` separator (they're typically UUID-like
/// or prefixed strings). Ed25519 keys are `base64.base64`.
pub fn is_polar_key(key: &str) -> bool {
    // Ed25519 keys always have exactly one `.` separating payload and signature.
    // Polar keys are typically alphanumeric with dashes, no dots.
    !key.contains('.')
}

/// Validate and activate a Polar license key against the Polar API.
///
/// This makes two API calls:
/// 1. `POST /validate` — checks the key is valid
/// 2. `POST /activate` — activates it for this machine
///
/// On success, caches the result locally so subsequent runs don't need network.
///
/// # Errors
///
/// Returns an error if the key is invalid, expired, or the API is unreachable.
pub async fn validate_polar_key(key: &str) -> Result<License> {
    let client = reqwest::Client::new();

    // Step 1: Validate the key.
    let validate_resp = client
        .post(format!("{POLAR_API_BASE}/validate"))
        .json(&serde_json::json!({
            "key": key,
            "organization_id": POLAR_ORG_ID,
        }))
        .send()
        .await
        .context("failed to reach Polar API — check your internet connection")?;

    if !validate_resp.status().is_success() {
        let status = validate_resp.status();
        let body = validate_resp.text().await.unwrap_or_default();
        bail!("Polar license validation failed (HTTP {status}): {body}");
    }

    let validate_data: PolarValidateResponse = validate_resp
        .json()
        .await
        .context("failed to parse Polar validation response")?;

    if validate_data.status != "granted" && validate_data.status != "active" {
        bail!(
            "Polar license key is not active (status: {})",
            validate_data.status
        );
    }

    // Step 2: Activate the key for this machine.
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "zvault-cli".to_owned());

    let activate_resp = client
        .post(format!("{POLAR_API_BASE}/activate"))
        .json(&serde_json::json!({
            "key": key,
            "organization_id": POLAR_ORG_ID,
            "label": hostname,
        }))
        .send()
        .await
        .context("failed to activate Polar license key")?;

    // 200 = newly activated, 4xx might mean already activated (which is fine).
    if activate_resp.status().is_success() {
        let _activate_data: PolarActivateResponse = activate_resp
            .json()
            .await
            .context("failed to parse Polar activation response")?;
    }

    // Map Polar benefit/product to tier based on known product IDs.
    let tier = match validate_data.benefit_id.as_deref() {
        Some("c42a3bec-5db8-4cf2-b9c6-48416604353e") => Tier::Team,
        Some("a2aaaded-328e-4320-a493-76bf7b898e45") => Tier::Enterprise,
        _ => Tier::Pro, // Default to Pro for unrecognized benefits
    };

    let now = now_iso8601();

    // Build a License object compatible with the rest of the system.
    let payload = LicensePayload {
        tier,
        email: validate_data
            .customer_id
            .unwrap_or_else(|| "polar-customer".to_owned()),
        issued_at: now.clone(),
        expires_at: validate_data
            .expires_at
            .clone()
            .unwrap_or_else(|| "2099-12-31T23:59:59Z".to_owned()),
        license_id: validate_data.id.clone(),
    };

    // Cache locally so the CLI works offline after activation.
    let cache = PolarLicenseCache {
        key: key.to_owned(),
        tier,
        validated_at: now,
        expires_at: validate_data.expires_at,
        license_id: validate_data.id,
    };
    save_polar_cache(&cache)?;

    Ok(License {
        payload,
        raw_key: key.to_owned(),
    })
}

/// Save Polar license cache to `~/.zvault/polar-license.json`.
fn save_polar_cache(cache: &PolarLicenseCache) -> Result<()> {
    let home = home_dir()?;
    let dir = home.join(".zvault");
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let path = dir.join("polar-license.json");
    let json =
        serde_json::to_string_pretty(cache).context("failed to serialize Polar license cache")?;
    std::fs::write(&path, &json).with_context(|| format!("failed to write {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    Ok(())
}

/// Load cached Polar license from `~/.zvault/polar-license.json`.
///
/// Returns `None` if no cache file exists.
fn load_polar_cache() -> Result<Option<PolarLicenseCache>> {
    let home = home_dir()?;
    let path = home.join(".zvault").join("polar-license.json");

    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let cache: PolarLicenseCache = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    Ok(Some(cache))
}

// ── License storage ──────────────────────────────────────────────────

/// Save a license key to the user-global license file (`~/.zvault/license.key`).
///
/// Sets file permissions to 0600 (owner read/write only) on Unix systems
/// to prevent other users from planting a crafted license file.
///
/// # Errors
///
/// Returns an error if the directory cannot be created or the file cannot be written.
pub fn save_license(key: &str) -> Result<PathBuf> {
    let home = home_dir()?;
    let dir = home.join(".zvault");
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let path = dir.join("license.key");
    std::fs::write(&path, key.trim())
        .with_context(|| format!("failed to write {}", path.display()))?;

    // Restrict permissions to owner-only (0600) on Unix.
    // Prevents other users on the machine from reading or replacing the license.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    Ok(path)
}

/// Load and verify the currently stored license.
///
/// Checks both Ed25519 license files and Polar license cache.
/// Returns `None` if no license file exists (Free tier).
///
/// On Unix, rejects license files that are world-writable or owned by
/// a different user (prevents privilege escalation via planted files).
///
/// # Errors
///
/// Returns an error if the license file exists but is invalid or expired.
pub fn load_license() -> Result<Option<License>> {
    // Try Ed25519 license first.
    let ed25519_result = load_ed25519_license();
    if let Ok(Some(lic)) = &ed25519_result {
        return Ok(Some(lic.clone()));
    }

    // Try Polar cache.
    if let Ok(Some(cache)) = load_polar_cache() {
        // Check if cached license has expired.
        if let Some(ref expires) = cache.expires_at {
            let now = now_iso8601();
            if *expires < now {
                // Expired — user needs to re-validate.
                return Ok(None);
            }
        }

        let payload = LicensePayload {
            tier: cache.tier,
            email: "polar-customer".to_owned(),
            issued_at: cache.validated_at.clone(),
            expires_at: cache
                .expires_at
                .unwrap_or_else(|| "2099-12-31T23:59:59Z".to_owned()),
            license_id: cache.license_id,
        };

        return Ok(Some(License {
            payload,
            raw_key: cache.key,
        }));
    }

    // If Ed25519 had an error (not just "no file"), propagate it.
    ed25519_result
}

/// Load and verify an Ed25519-signed license file.
fn load_ed25519_license() -> Result<Option<License>> {
    let path = license_file_path()?;

    if !path.exists() {
        return Ok(None);
    }

    // On Unix, verify the license file isn't world-writable or owned by
    // someone else. This prevents a local attacker from planting a crafted
    // license file in a shared directory.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(&path)
            .with_context(|| format!("failed to stat {}", path.display()))?;

        // Reject world-writable files (mode & 0o002).
        if meta.mode() & 0o002 != 0 {
            bail!(
                "license file {} is world-writable — refusing to load (security risk)",
                path.display()
            );
        }

        // Reject files owned by a different user.
        let file_uid = meta.uid();
        let my_uid = unsafe_geteuid();
        if file_uid != my_uid && my_uid != 0 {
            bail!(
                "license file {} is owned by uid {} but we are uid {} — refusing to load",
                path.display(),
                file_uid,
                my_uid
            );
        }
    }

    let key = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let key = key.trim();
    if key.is_empty() {
        return Ok(None);
    }

    let license = verify_license_key(key)?;
    Ok(Some(license))
}

/// Get the effective user ID on Unix.
///
/// This is a thin wrapper around `libc::geteuid()`. We use it to verify
/// license file ownership.
#[cfg(unix)]
fn unsafe_geteuid() -> u32 {
    // SAFETY: `geteuid()` is a simple syscall with no preconditions.
    // It cannot fail and has no side effects.
    #[allow(unsafe_code)]
    unsafe {
        libc::geteuid()
    }
}

/// Get the current license tier. Returns `Free` if no license is installed.
pub fn current_tier() -> Tier {
    match load_license() {
        Ok(Some(lic)) => lic.payload.tier,
        _ => Tier::Free,
    }
}

// ── Feature gating ───────────────────────────────────────────────────

/// Check if the current license allows a Pro+ feature.
///
/// # Errors
///
/// Returns an error with a user-friendly message if the feature requires
/// a higher tier than the current license.
pub fn require_pro(feature_name: &str) -> Result<()> {
    // Dev bypass: skip license check during development.
    if std::env::var("ZVAULT_DEV").is_ok() {
        return Ok(());
    }

    let tier = current_tier();
    if tier >= Tier::Pro {
        return Ok(());
    }

    bail!(
        "{feature_name} requires a Pro license ($8/mo).\n\
         \n  \
         Upgrade at https://zvault.cloud/pricing\n  \
         Then run: zvault activate <license-key>\n"
    );
}

/// Check if the current license allows a Team+ feature.
///
/// # Errors
///
/// Returns an error with a user-friendly message if the feature requires
/// a higher tier than the current license.
#[allow(dead_code)]
pub fn require_team(feature_name: &str) -> Result<()> {
    let tier = current_tier();
    if tier >= Tier::Team {
        return Ok(());
    }

    bail!(
        "{feature_name} requires a Team license ($19/mo).\n\
         \n  \
         Upgrade at https://zvault.cloud/pricing\n  \
         Then run: zvault activate <license-key>\n"
    );
}

/// Check if the current license allows an Enterprise feature.
///
/// # Errors
///
/// Returns an error with a user-friendly message if the feature requires
/// a higher tier than the current license.
#[allow(dead_code)]
pub fn require_enterprise(feature_name: &str) -> Result<()> {
    let tier = current_tier();
    if tier >= Tier::Enterprise {
        return Ok(());
    }

    bail!(
        "{feature_name} requires an Enterprise license ($49/mo).\n\
         \n  \
         Contact sales@zvault.cloud for pricing.\n"
    );
}
