//! PKI secrets engine for `ZVault`.
//!
//! Generates a self-signed root CA and issues X.509 certificates on demand.
//! Certificates are tracked via the lease system. Uses `rcgen` for pure-Rust
//! certificate generation — no OpenSSL dependency.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::barrier::Barrier;
use crate::error::PkiError;

/// Root CA data stored in the barrier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaData {
    /// PEM-encoded CA certificate.
    pub certificate_pem: String,
    /// PEM-encoded CA private key (encrypted at rest via barrier).
    pub private_key_pem: String,
    /// Subject common name.
    pub common_name: String,
    /// Validity period in hours.
    pub ttl_hours: u64,
}

/// A PKI role that controls certificate issuance parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkiRole {
    /// Role name.
    pub name: String,
    /// Allowed domains for issued certificates.
    pub allowed_domains: Vec<String>,
    /// Whether subdomains of `allowed_domains` are permitted.
    pub allow_subdomains: bool,
    /// Maximum TTL in hours for issued certificates.
    pub max_ttl_hours: u64,
    /// Whether to generate the private key server-side.
    pub generate_key: bool,
    /// Key type: "rsa" or "ec".
    pub key_type: String,
    /// Key bits (2048, 4096 for RSA; 256, 384 for EC).
    pub key_bits: u32,
}

/// An issued certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedCertificate {
    /// PEM-encoded certificate.
    pub certificate_pem: String,
    /// PEM-encoded private key (if generated server-side).
    pub private_key_pem: Option<String>,
    /// PEM-encoded CA certificate chain.
    pub ca_chain_pem: String,
    /// Serial number (hex).
    pub serial_number: String,
    /// Expiration timestamp (RFC 3339).
    pub expiration: String,
}

/// The PKI secrets engine.
pub struct PkiEngine {
    barrier: Arc<Barrier>,
    prefix: String,
    /// Cached CA data.
    ca: RwLock<Option<CaData>>,
    /// Cached roles.
    roles: RwLock<HashMap<String, PkiRole>>,
}

impl PkiEngine {
    /// Create a new PKI engine with the given barrier and storage prefix.
    pub fn new(barrier: Arc<Barrier>, prefix: String) -> Self {
        Self {
            barrier,
            prefix,
            ca: RwLock::new(None),
            roles: RwLock::new(HashMap::new()),
        }
    }

    fn ca_key(&self) -> String {
        format!("{}ca/root", self.prefix)
    }

    fn role_key(&self, name: &str) -> String {
        format!("{}roles/{}", self.prefix, name)
    }

    fn cert_key(&self, serial: &str) -> String {
        format!("{}certs/{}", self.prefix, serial)
    }

    /// Generate a self-signed root CA.
    ///
    /// # Errors
    ///
    /// Returns `PkiError::InvalidRequest` if `common_name` is empty.
    /// Returns `PkiError::CertGeneration` if certificate generation fails.
    pub async fn generate_root(
        &self,
        common_name: &str,
        ttl_hours: u64,
    ) -> Result<CaData, PkiError> {
        if common_name.is_empty() {
            return Err(PkiError::InvalidRequest {
                reason: "common_name is required".to_owned(),
            });
        }

        let params = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(|e| {
            PkiError::CertGeneration {
                reason: format!("failed to create cert params: {e}"),
            }
        })?;

        let key_pair = rcgen::KeyPair::generate().map_err(|e| PkiError::CertGeneration {
            reason: format!("key generation failed: {e}"),
        })?;

        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| PkiError::CertGeneration {
                reason: format!("self-signing failed: {e}"),
            })?;

        let ca_data = CaData {
            certificate_pem: cert.pem(),
            private_key_pem: key_pair.serialize_pem(),
            common_name: common_name.to_owned(),
            ttl_hours,
        };

        let data = serde_json::to_vec(&ca_data).map_err(|e| PkiError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier.put(&self.ca_key(), &data).await?;
        *self.ca.write().await = Some(ca_data.clone());

        Ok(ca_data)
    }

    /// Get the current root CA.
    ///
    /// # Errors
    ///
    /// Returns `PkiError::NoRootCa` if no CA has been generated.
    pub async fn get_ca(&self) -> Result<CaData, PkiError> {
        if let Some(ca) = self.ca.read().await.as_ref() {
            return Ok(ca.clone());
        }
        let data = self
            .barrier
            .get(&self.ca_key())
            .await?
            .ok_or(PkiError::NoRootCa)?;
        let ca: CaData = serde_json::from_slice(&data).map_err(|e| PkiError::Internal {
            reason: format!("deserialization failed: {e}"),
        })?;
        *self.ca.write().await = Some(ca.clone());
        Ok(ca)
    }

    /// Create a PKI role.
    ///
    /// # Errors
    ///
    /// Returns `PkiError::InvalidRequest` if required fields are missing.
    pub async fn create_role(&self, role: PkiRole) -> Result<(), PkiError> {
        if role.name.is_empty() {
            return Err(PkiError::InvalidRequest {
                reason: "role name is required".to_owned(),
            });
        }
        if role.allowed_domains.is_empty() {
            return Err(PkiError::InvalidRequest {
                reason: "allowed_domains is required".to_owned(),
            });
        }
        let data = serde_json::to_vec(&role).map_err(|e| PkiError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier.put(&self.role_key(&role.name), &data).await?;
        self.roles.write().await.insert(role.name.clone(), role);
        Ok(())
    }

    /// Get a PKI role by name.
    ///
    /// # Errors
    ///
    /// Returns `PkiError::RoleNotFound` if the role does not exist.
    pub async fn get_role(&self, name: &str) -> Result<PkiRole, PkiError> {
        if let Some(role) = self.roles.read().await.get(name) {
            return Ok(role.clone());
        }
        let data = self
            .barrier
            .get(&self.role_key(name))
            .await?
            .ok_or_else(|| PkiError::RoleNotFound {
                name: name.to_owned(),
            })?;
        let role: PkiRole = serde_json::from_slice(&data).map_err(|e| PkiError::Internal {
            reason: format!("deserialization failed: {e}"),
        })?;
        self.roles
            .write()
            .await
            .insert(name.to_owned(), role.clone());
        Ok(role)
    }

    /// List all PKI role names.
    ///
    /// # Errors
    ///
    /// Returns `PkiError::Barrier` if the barrier is sealed.
    pub async fn list_roles(&self) -> Result<Vec<String>, PkiError> {
        let prefix = format!("{}roles/", self.prefix);
        let keys = self.barrier.list(&prefix).await?;
        Ok(keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(String::from))
            .collect())
    }

    /// Issue a certificate for the given common name using a role.
    ///
    /// # Errors
    ///
    /// Returns `PkiError::NoRootCa` if no CA exists.
    /// Returns `PkiError::RoleNotFound` if the role does not exist.
    /// Returns `PkiError::InvalidRequest` if the domain is not allowed.
    pub async fn issue(
        &self,
        role_name: &str,
        common_name: &str,
        ttl_hours: Option<u64>,
    ) -> Result<IssuedCertificate, PkiError> {
        let ca = self.get_ca().await?;
        let role = self.get_role(role_name).await?;

        // Validate domain against allowed_domains.
        let domain_allowed = role.allowed_domains.iter().any(|d| {
            common_name == d.as_str()
                || (role.allow_subdomains && common_name.ends_with(&format!(".{d}")))
        });
        if !domain_allowed {
            return Err(PkiError::InvalidRequest {
                reason: format!("domain '{common_name}' not allowed by role '{role_name}'"),
            });
        }

        let effective_ttl = ttl_hours
            .unwrap_or(role.max_ttl_hours)
            .min(role.max_ttl_hours);

        // Parse CA key pair.
        let ca_key_pair = rcgen::KeyPair::from_pem(&ca.private_key_pem).map_err(|e| {
            PkiError::CertGeneration {
                reason: format!("failed to parse CA key: {e}"),
            }
        })?;

        let ca_params = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(|e| {
            PkiError::CertGeneration {
                reason: format!("failed to create CA params: {e}"),
            }
        })?;
        let ca_cert =
            ca_params
                .self_signed(&ca_key_pair)
                .map_err(|e| PkiError::CertGeneration {
                    reason: format!("failed to reconstruct CA cert: {e}"),
                })?;

        // Generate leaf certificate.
        let leaf_params =
            rcgen::CertificateParams::new(vec![common_name.to_owned()]).map_err(|e| {
                PkiError::CertGeneration {
                    reason: format!("failed to create leaf params: {e}"),
                }
            })?;

        let leaf_key = rcgen::KeyPair::generate().map_err(|e| PkiError::CertGeneration {
            reason: format!("leaf key generation failed: {e}"),
        })?;

        let leaf_cert = leaf_params
            .signed_by(&leaf_key, &ca_cert, &ca_key_pair)
            .map_err(|e| PkiError::CertGeneration {
                reason: format!("certificate signing failed: {e}"),
            })?;

        let serial = uuid::Uuid::new_v4().to_string().replace('-', "");
        let effective_ttl_i64 = i64::try_from(effective_ttl).unwrap_or(i64::MAX);
        let expiration = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(effective_ttl_i64))
            .map(|t| t.to_rfc3339())
            .unwrap_or_default();

        let issued = IssuedCertificate {
            certificate_pem: leaf_cert.pem(),
            private_key_pem: if role.generate_key {
                Some(leaf_key.serialize_pem())
            } else {
                None
            },
            ca_chain_pem: ca.certificate_pem.clone(),
            serial_number: serial.clone(),
            expiration,
        };

        // Store issued cert metadata.
        let cert_data = serde_json::to_vec(&issued).map_err(|e| PkiError::Internal {
            reason: format!("serialization failed: {e}"),
        })?;
        self.barrier
            .put(&self.cert_key(&serial), &cert_data)
            .await?;

        Ok(issued)
    }

    /// List all issued certificate serial numbers.
    ///
    /// # Errors
    ///
    /// Returns `PkiError::Barrier` if the barrier is sealed.
    pub async fn list_certs(&self) -> Result<Vec<String>, PkiError> {
        let prefix = format!("{}certs/", self.prefix);
        let keys = self.barrier.list(&prefix).await?;
        Ok(keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(String::from))
            .collect())
    }
}
