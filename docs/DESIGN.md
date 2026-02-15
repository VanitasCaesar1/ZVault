# ZVault — Complete Design Document

An in-house secrets management platform written in Rust. Takes the security
architecture of HashiCorp Vault and the developer ergonomics of Infisical,
ships as a single static binary with an embedded storage engine.

---

## 1. Research: What We Learned

### 1.1 HashiCorp Vault

Vault is the gold standard. The things that matter:

**Barrier pattern.** Every byte that touches storage is encrypted. The storage
backend is a dumb bucket of ciphertext. This is non-negotiable — we adopt this.

**Seal/Unseal via Shamir's Secret Sharing.** A 256-bit root key encrypts all
data. That root key is itself encrypted by an "unseal key" which is split into
N shares with a threshold of T. Operators must provide T shares after every
restart to reconstruct the unseal key and decrypt the root key into memory.
The root key never touches disk in plaintext. We adopt this.

**Secrets engines as a plugin system.** Vault's killer feature is that "secrets"
aren't just static KV pairs. Engines can *generate* credentials on the fly:

- **KV engine**: Static key-value secrets with versioning
- **Database engine**: Connects to a target DB (Postgres, MySQL, etc.), creates
  temporary credentials with a TTL, revokes them on expiry
- **PKI engine**: Acts as a certificate authority — generates X.509 certs on
  demand, handles CRL, OCSP
- **Transit engine**: Encryption-as-a-service — apps send plaintext, get
  ciphertext back. Vault holds the keys, apps never see them
- **Cloud engines**: Generate short-lived IAM credentials for AWS/GCP/Azure

We adopt the engine abstraction as a trait system in Rust.

**Token auth + pluggable auth methods.** Vault supports tokens, LDAP, K8s
service accounts, OIDC, AWS IAM, and more. Policies are path-based HCL rules.

**Raft-based HA.** Vault's integrated storage uses Raft consensus for leader
election and data replication across a 3-5 node cluster. No external
dependency (replaced Consul). We adopt this approach.

**Immutable audit log.** Every operation is logged. Logs include HMAC'd
sensitive fields so you can verify without exposing secrets.

### 1.2 Infisical

Infisical is newer, simpler, developer-first:

- **Postgres as the single backend** — no Consul, no Raft, just a database.
  Works great for small-to-medium teams. But doesn't scale for HA without
  external Postgres replication.
- **Folder/environment hierarchy** — secrets organized as
  `project/environment/folder/key`. Maps naturally to dev/staging/prod.
- **Simple RBAC** — permissions at the folder level (read/write/admin) instead
  of Vault's HCL policy language.
- **End-to-end encryption** — secrets encrypted client-side before reaching
  the server. Nice for zero-trust, but complicates server-side operations
  like rotation.
- **Redis for async jobs** — rotation, syncing, webhooks run through a queue.
- **REST API + CLI + SDKs + K8s operator** — good client ecosystem.

### 1.3 What We Take From Each

| Feature | Source | Priority |
|---|---|---|
| Encryption barrier | Vault | Core |
| Shamir seal/unseal | Vault | Core |
| Secrets engine trait system | Vault | Core |
| KV engine with versioning | Both | Core |
| Database dynamic credentials | Vault | Core |
| Transit encryption-as-a-service | Vault | Core |
| PKI / X.509 certificate engine | Vault | Core |
| Token + OIDC auth | Vault | Core |
| Path-based RBAC (simplified) | Infisical | Core |
| Folder/environment hierarchy | Infisical | Core |
| Immutable audit log (HMAC'd) | Vault | Core |
| Raft HA clustering | Vault | Core |
| Embedded storage (no external DB) | Design choice | Core |
| CLI client | Both | Core |
| K8s operator (CRD-based sync) | Both | Core |
| Secret rotation scheduler | Infisical | Core |
| Prometheus metrics | Standard | Core |

---

## 2. Storage: RocksDB vs Postgres vs Embedded Alternatives

This is the most consequential architectural decision. Let's be thorough.

### 2.1 Option A: PostgreSQL (what Infisical does)

**Pros:**
- Rich query capabilities (audit log filtering, secret listing, JSONB policies)
- Mature replication for HA (streaming replication, Patroni)
- Familiar to most ops teams
- Transactions, constraints, indexes

**Cons:**
- External dependency — you need to run and maintain a Postgres cluster
- Defeats the "single binary" goal
- Connection pooling, backups, upgrades are your problem
- Overkill for what is fundamentally a key-value workload

### 2.2 Option B: RocksDB (embedded LSM-tree)

**Pros:**
- Embedded — compiles into the binary (via `rust-rocksdb` crate, C++ FFI)
- Extremely fast for key-value workloads (millions of ops/sec)
- Battle-tested at Facebook, TiKV, CockroachDB scale
- Column families for logical separation (secrets, audit, config, leases)
- Supports snapshots, compression, TTL

**Cons:**
- C++ dependency — requires linking against librocksdb (complicates cross-compilation)
- LSM-tree means write amplification and compaction pauses
- No built-in query language — you build your own indexing
- Tuning knobs are complex (memtable size, compaction strategy, bloom filters)

### 2.3 Option C: redb (pure Rust embedded B-tree)

**Pros:**
- Pure Rust — no C/C++ FFI, trivial cross-compilation
- ACID transactions
- Simple API, small footprint
- B-tree means consistent read/write performance (no compaction storms)

**Cons:**
- Much younger project than RocksDB
- Smaller community, fewer production deployments
- No column families (use separate tables)
- Lower raw throughput than RocksDB for write-heavy workloads

### 2.4 Decision: Pluggable Storage Backend with RocksDB as Default

We use a **storage backend trait** so the engine is swappable, but ship with
two implementations:

1. **RocksDB** (default) — for production. Best performance, proven at scale.
   The C++ dependency is acceptable since we're not targeting WASM.
2. **redb** (optional, feature flag) — for development, testing, and
   environments where pure-Rust compilation matters.

This mirrors Vault's approach (pluggable backends) without the operational
burden of external systems. The binary embeds its own storage.

For **HA mode**, we layer Raft consensus on top of the storage backend. The
leader accepts writes, replicates the encrypted WAL to followers. This is
exactly what Vault does with its integrated Raft storage.

```
Single-node:  Client → API → Barrier → RocksDB (local)
HA cluster:   Client → API → Barrier → Raft Leader → RocksDB (replicated)
```

---

## 3. Architecture

### 3.1 High-Level Overview

```
┌──────────────────────────────────────────────────────────────┐
│                        Clients                               │
│     CLI  ·  SDK (Rust/Go/Python/Node)  ·  K8s Operator      │
│     curl  ·  CI/CD plugins  ·  Web UI                        │
└────────────────────────┬─────────────────────────────────────┘
                         │ mTLS / HTTPS
                         ▼
┌──────────────────────────────────────────────────────────────┐
│                     ZVault Node                             │
│                                                              │
│  ┌──────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐ │
│  │  Auth    │  │  Router   │  │  Audit    │  │  Metrics  │ │
│  │  Layer   │  │  (Axum)   │  │  Logger   │  │  (Prom)   │ │
│  └────┬─────┘  └─────┬─────┘  └─────┬─────┘  └───────────┘ │
│       │              │              │                        │
│       ▼              ▼              ▼                        │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Secrets Engine Router                    │   │
│  │                                                      │   │
│  │  ┌─────┐  ┌──────────┐  ┌─────────┐  ┌───────────┐ │   │
│  │  │ KV  │  │ Database │  │ Transit │  │    PKI    │ │   │
│  │  │ v2  │  │ (dynamic)│  │ (EaaS)  │  │ (X.509)  │ │   │
│  │  └─────┘  └──────────┘  └─────────┘  └───────────┘ │   │
│  └──────────────────────┬───────────────────────────────┘   │
│                         │                                    │
│                ┌────────▼────────┐                           │
│                │   Encryption    │  ← root key in RAM only   │
│                │   Barrier       │                           │
│                │  (AES-256-GCM)  │                           │
│                └────────┬────────┘                           │
│                         │ ciphertext only below this line    │
│                ┌────────▼────────┐                           │
│                │  Storage Trait  │                           │
│                │  impl: RocksDB  │                           │
│                │  impl: redb    │                           │
│                └────────┬────────┘                           │
│                         │                                    │
│  ┌──────────────────────▼───────────────────────────────┐   │
│  │              Raft Consensus (HA mode)                 │   │
│  │         Leader election · Log replication             │   │
│  │              Snapshot · Membership changes            │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 Key Hierarchy

```
Unseal Key (256-bit)
  │  Split into N shares via Shamir's Secret Sharing
  │  Reconstructed from T shares at unseal time
  │  NEVER stored anywhere — lives only in operator hands
  │
  ├──► Encrypts: Root Key
  │
  ▼
Root Key (256-bit)
  │  Encrypted by unseal key, stored in storage backend
  │  Decrypted at unseal, held in process memory only
  │  Zeroed on seal/shutdown (zeroize crate)
  │
  ├──► Encrypts: All KV secret values
  ├──► Encrypts: All engine configuration (DB connection strings, etc.)
  ├──► Encrypts: Transit engine key material
  ├──► Encrypts: PKI private keys
  │
  ▼
Per-Engine Keys (derived from root key via HKDF)
  │  Each secrets engine gets its own derived key
  │  Limits blast radius if one engine is compromised
  │
  ▼
Data Encryption Keys (DEKs, for transit engine)
  │  Named keys that clients reference
  │  Support key rotation (versioned)
  │  Encrypted by the transit engine's derived key
```

### 3.3 Lifecycle

```
┌─────────┐     ┌──────────┐     ┌───────────┐     ┌────────┐
│  INIT   │────►│  SEALED  │────►│ UNSEALED  │────►│ SEALED │
│ (once)  │     │          │     │ (serving) │     │        │
└─────────┘     └──────────┘     └───────────┘     └────────┘
                  ▲    │              │                  │
                  │    │  T shares    │   seal cmd /     │
                  │    └──────────────┘   restart        │
                  │                                      │
                  └──────────────────────────────────────┘

INIT:
  1. Generate root key (256-bit random)
  2. Generate unseal key (256-bit random)
  3. Encrypt root key with unseal key → store in backend
  4. Split unseal key into N shares (Shamir) → return to operator
  5. Generate root token → hash and store
  6. State = SEALED

UNSEAL:
  1. Operator submits shares one at a time (or batch)
  2. Server accumulates shares in memory
  3. When threshold T reached → reconstruct unseal key
  4. Decrypt root key → hold in memory
  5. Derive per-engine keys via HKDF
  6. State = UNSEALED

SEAL:
  1. Zeroize root key and all derived keys from memory
  2. Reject all secret operations
  3. State = SEALED (requires unseal again)
```

---

## 4. Secrets Engines

### 4.1 Engine Trait

Every secrets engine implements a common trait:

```rust
#[async_trait]
pub trait SecretsEngine: Send + Sync {
    /// Engine type identifier (e.g., "kv", "database", "transit", "pki")
    fn engine_type(&self) -> &str;

    /// Handle a read operation
    async fn read(&self, path: &str, ctx: &RequestContext) -> Result<Value>;

    /// Handle a write operation
    async fn write(&self, path: &str, data: Value, ctx: &RequestContext) -> Result<Value>;

    /// Handle a delete operation
    async fn delete(&self, path: &str, ctx: &RequestContext) -> Result<()>;

    /// Handle a list operation
    async fn list(&self, prefix: &str, ctx: &RequestContext) -> Result<Vec<String>>;

    /// Called when the engine is mounted — initialize state
    async fn init(&mut self, config: Value, barrier: Arc<Barrier>) -> Result<()>;

    /// Called periodically — handle lease expiry, CRL rebuild, etc.
    async fn tick(&self) -> Result<()>;

    /// Clean shutdown
    async fn shutdown(&self) -> Result<()>;
}
```

Engines are mounted at paths: `secret/` (KV), `database/` (dynamic),
`transit/` (encryption), `pki/` (certificates). The router dispatches
requests to the correct engine based on the path prefix.

### 4.2 KV Engine (v2, versioned)

The bread and butter. Static secrets with full version history.

**Storage layout** (keys in RocksDB):
```
kv/<mount>/data/<path>          → encrypted JSON { value, version, metadata }
kv/<mount>/metadata/<path>      → { versions: [...], current_version, max_versions }
kv/<mount>/delete/<path>/<ver>  → soft-delete marker
```

**Features:**
- Read latest or specific version
- Write creates a new version (append-only)
- Soft delete (recoverable) and hard destroy
- Configurable max versions per secret
- CAS (check-and-set) writes to prevent race conditions
- Metadata (custom key-value pairs per secret)

### 4.3 Database Engine (Dynamic Credentials)

Generates short-lived database credentials on demand. This is Vault's most
powerful feature and the main reason teams adopt it.

**How it works:**
1. Admin configures a "connection" — target DB host, port, admin credentials
2. Admin creates "roles" — SQL templates for creating users
3. Client requests credentials for a role
4. Engine connects to target DB, executes the creation SQL
5. Returns temporary username/password with a TTL (lease)
6. On lease expiry, engine connects and executes revocation SQL

**Supported databases (via connection plugins):**
- PostgreSQL (`CREATE ROLE ... LOGIN PASSWORD ... VALID UNTIL ...`)
- MySQL (`CREATE USER ... IDENTIFIED BY ... PASSWORD EXPIRE INTERVAL ...`)
- MongoDB (future)

**Storage layout:**
```
db/<mount>/config/<connection-name>  → encrypted connection config
db/<mount>/roles/<role-name>         → role definition (creation/revocation SQL, TTL)
db/<mount>/creds/<role-name>         → (virtual — generates on read)
```

**Lease management:**
```rust
pub struct Lease {
    pub id: String,           // unique lease ID
    pub engine_path: String,  // "database/creds/readonly"
    pub issued_at: DateTime<Utc>,
    pub ttl: Duration,
    pub renewable: bool,
    pub data: Value,          // engine-specific (e.g., username to revoke)
}
```

A background task (the "lease manager") runs on a tick interval, finds
expired leases, and calls the engine's revocation logic.

### 4.4 Transit Engine (Encryption as a Service)

Apps send plaintext, get ciphertext back. They never see the encryption keys.

**Operations:**
- `encrypt` — encrypt plaintext with a named key
- `decrypt` — decrypt ciphertext with a named key
- `rewrap` — re-encrypt ciphertext with the latest key version (key rotation)
- `sign` — sign data with a named key (Ed25519 or ECDSA)
- `verify` — verify a signature
- `generate-data-key` — return a new random DEK, optionally wrapped
- `hash` — compute SHA-256/SHA-512 of data
- `generate-random` — return random bytes

**Key types:**
- `aes256-gcm` — AES-256-GCM (default, symmetric)
- `ed25519` — Ed25519 signing
- `ecdsa-p256` — ECDSA P-256 signing
- `rsa-2048` / `rsa-4096` — RSA (signing + encryption)

**Key versioning:**
Each named key can have multiple versions. Encryption always uses the latest
version. Decryption tries all versions (ciphertext includes a version prefix).
Old versions can be disabled or destroyed.

**Storage layout:**
```
transit/<mount>/keys/<key-name>         → key metadata + encrypted key material
transit/<mount>/keys/<key-name>/<ver>   → versioned key material
```

### 4.5 PKI Engine (Certificate Authority)

Acts as an internal CA. Generates X.509 certificates on demand.

**Capabilities:**
- Generate self-signed root CA
- Generate intermediate CA (signed by root or external CA)
- Issue leaf certificates with configurable SANs, TTL, key usage
- Certificate Revocation List (CRL) generation
- OCSP responder (future)

**Roles** define templates for certificate issuance:
```json
{
  "name": "web-server",
  "allowed_domains": ["*.internal.company.com"],
  "allow_subdomains": true,
  "max_ttl": "8760h",
  "key_type": "ec",
  "key_bits": 256,
  "key_usage": ["DigitalSignature", "KeyEncipherment"],
  "ext_key_usage": ["ServerAuth"]
}
```

**Storage layout:**
```
pki/<mount>/ca/cert          → CA certificate (PEM)
pki/<mount>/ca/key           → encrypted CA private key
pki/<mount>/roles/<name>     → role configuration
pki/<mount>/certs/<serial>   → issued certificate
pki/<mount>/crl              → current CRL
```

**Rust crate:** `rcgen` for X.509 generation, `x509-parser` for parsing.

---

## 5. Authentication & Authorization

### 5.1 Auth Methods

Like Vault, auth is pluggable via a trait:

```rust
#[async_trait]
pub trait AuthMethod: Send + Sync {
    fn method_type(&self) -> &str;
    async fn authenticate(&self, credentials: Value) -> Result<AuthResponse>;
}

pub struct AuthResponse {
    pub token: String,
    pub policies: Vec<String>,
    pub ttl: Duration,
    pub renewable: bool,
    pub metadata: HashMap<String, String>,
}
```

**Built-in methods:**
- **Token**: Direct token auth (like Vault). Root token created at init.
  Service accounts get tokens bound to policies.
- **OIDC/OAuth2**: Authenticate via external identity provider (Okta, Auth0,
  Keycloak, Google). Map OIDC claims to policies.
- **Kubernetes**: Validate K8s service account JWTs. Map service accounts
  to policies. Essential for the K8s operator.
- **AppRole**: Machine-oriented auth. A role ID (public) + secret ID
  (private, single-use) produces a token. Good for CI/CD.

### 5.2 Policy System

Simplified from Vault's HCL. Policies are JSON documents:

```json
{
  "name": "app-readonly",
  "rules": [
    { "path": "secret/data/production/*", "capabilities": ["read", "list"] },
    { "path": "database/creds/readonly", "capabilities": ["read"] },
    { "path": "transit/encrypt/app-key", "capabilities": ["update"] },
    { "path": "transit/decrypt/app-key", "capabilities": ["update"] }
  ]
}
```

**Capabilities:** `read`, `list`, `create`, `update`, `delete`, `sudo` (admin ops), `deny`

**Path matching:**
- Exact: `secret/data/production/db-password`
- Glob: `secret/data/production/*` (one level)
- Recursive glob: `secret/data/production/**` (all descendants)
- `deny` always wins over other capabilities

### 5.3 Token Lifecycle

```
Token created → active (TTL counting down)
  │
  ├── renew → TTL reset (if renewable, up to max_ttl)
  │
  ├── expire → revoked automatically, all leases revoked
  │
  └── revoke → explicit revocation, all child tokens + leases revoked
```

Tokens form a tree. Revoking a parent revokes all children (like Vault's
token hierarchy). This prevents orphaned access.

---

## 6. Lease Manager

Central to dynamic secrets. Every dynamically generated credential gets a lease.

```rust
pub struct LeaseManager {
    storage: Arc<dyn StorageBackend>,
    engines: Arc<EngineRouter>,
}

impl LeaseManager {
    /// Runs every N seconds, finds expired leases, revokes them
    pub async fn tick(&self) -> Result<()>;

    /// Create a new lease
    pub async fn create(&self, lease: Lease) -> Result<String>;

    /// Renew an existing lease (extend TTL)
    pub async fn renew(&self, lease_id: &str, increment: Duration) -> Result<Lease>;

    /// Revoke a lease immediately
    pub async fn revoke(&self, lease_id: &str) -> Result<()>;

    /// Revoke all leases for a prefix (e.g., when unmounting an engine)
    pub async fn revoke_prefix(&self, prefix: &str) -> Result<u64>;
}
```

**Storage layout:**
```
sys/leases/<lease-id>  → { engine_path, issued_at, ttl, data }
sys/leases/by-token/<token-id>/<lease-id>  → pointer (for token revocation)
sys/leases/by-expiry/<timestamp>/<lease-id>  → pointer (for tick scan)
```

---

## 7. Audit System

### 7.1 Audit Log

Every API request generates an audit entry:

```rust
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub request: AuditRequest,
    pub response: AuditResponse,
    pub auth: AuditAuth,
}

pub struct AuditRequest {
    pub operation: String,      // "read", "write", "delete", "login"
    pub path: String,           // "secret/data/production/db-password"
    pub data: Option<Value>,    // HMAC'd sensitive fields
    pub remote_addr: String,
}

pub struct AuditResponse {
    pub status_code: u16,
    pub error: Option<String>,
}

pub struct AuditAuth {
    pub token_id: String,       // HMAC'd
    pub policies: Vec<String>,
    pub metadata: HashMap<String, String>,
}
```

**Sensitive field handling:** Secret values in request/response data are
HMAC'd with a per-audit-device key. This lets you search and correlate
audit entries without exposing actual secret values.

### 7.2 Audit Backends

Pluggable via trait:

```rust
#[async_trait]
pub trait AuditBackend: Send + Sync {
    async fn log(&self, entry: &AuditEntry) -> Result<()>;
}
```

**Built-in backends:**
- **File**: Append-only JSON lines to a file
- **Storage**: Write to the embedded storage backend
- **Syslog**: Forward to syslog (future)

**Fail-open vs fail-closed:** If ALL audit backends fail to write, the
request is denied. This ensures no operation goes unaudited (Vault does this).

---

## 8. HA Clustering

### 8.1 Raft Consensus

For high availability, we use the Raft consensus protocol via the `openraft`
crate (async Rust, 70k+ writes/sec, CNCF-adjacent ecosystem).

**Cluster topology:**
- 3 or 5 nodes (odd number for quorum)
- One leader, rest are followers
- Leader handles all writes, replicates to followers
- Followers can serve stale reads (configurable consistency)
- Automatic leader election on failure

**What gets replicated:**
- All storage backend writes (encrypted, below the barrier)
- Seal/unseal state
- Engine mount table
- Policy definitions

**What does NOT get replicated:**
- The root key (each node must be unsealed independently)
- In-memory caches
- Active connections to external systems (DB engines)

### 8.2 Cluster Architecture

```
                    ┌─────────────┐
         ┌────────►│   Node 1    │◄────────┐
         │         │  (Leader)   │         │
         │         │  RocksDB    │         │
         │         └──────┬──────┘         │
         │                │                │
    Raft replication  Raft replication  Raft replication
         │                │                │
         ▼                ▼                ▼
  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
  │   Node 2     │ │   Node 3     │ │   Node 4     │
  │  (Follower)  │ │  (Follower)  │ │  (Follower)  │
  │  RocksDB     │ │  RocksDB     │ │  RocksDB     │
  └──────────────┘ └──────────────┘ └──────────────┘

Client requests → Load balancer → Any node
  - Writes forwarded to leader
  - Reads served locally (eventual) or forwarded (strong)
```

### 8.3 Node Configuration

```toml
# zvault.toml
[server]
bind_addr = "0.0.0.0:8200"
tls_cert = "/etc/zvault/tls/cert.pem"
tls_key = "/etc/zvault/tls/key.pem"

[storage]
backend = "rocksdb"           # or "redb"
path = "/var/lib/zvault/data"

[cluster]
enabled = true
node_id = "node-1"
bind_addr = "10.0.1.1:8201"  # inter-node communication
peers = [
    "10.0.1.2:8201",
    "10.0.1.3:8201",
]

[telemetry]
prometheus_bind = "0.0.0.0:9090"
```

---

## 9. Kubernetes Operator

A separate binary (`zvault-operator`) that runs in-cluster and syncs
secrets from ZVault into Kubernetes Secrets.

### 9.1 Custom Resource Definitions

```yaml
apiVersion: zvault.io/v1alpha1
kind: ZVaultSecret
metadata:
  name: db-credentials
  namespace: production
spec:
  # Where to read from ZVault
  secretPath: "secret/data/production/db-password"
  # Or dynamic credentials
  # secretPath: "database/creds/readonly"

  # How to authenticate with ZVault
  authRef:
    method: kubernetes
    role: "production-app"

  # Target K8s secret
  target:
    name: db-credentials
    type: Opaque
    template:
      data:
        username: "{{ .Data.username }}"
        password: "{{ .Data.password }}"

  # Refresh interval (for dynamic secrets)
  refreshInterval: 60s
```

### 9.2 Operator Architecture

Built with `kube-rs` (CNCF Sandbox project):

```
┌─────────────────────────────────────────┐
│          zvault-operator               │
│                                         │
│  ┌─────────────┐  ┌─────────────────┐  │
│  │  Controller  │  │  ZVault Client │  │
│  │  (kube-rs)   │──│  (HTTP + auth)  │  │
│  └──────┬──────┘  └────────┬────────┘  │
│         │                   │           │
│    Watch CRDs          Fetch secrets    │
│    Reconcile loop      Renew leases     │
│    Create/Update       Handle rotation  │
│    K8s Secrets                          │
└─────────────────────────────────────────┘
```

The reconciliation loop:
1. Watch for `ZVaultSecret` CRD changes
2. Authenticate with ZVault using configured auth method
3. Fetch secret from ZVault
4. Create/update target Kubernetes Secret
5. Re-queue for refresh (based on `refreshInterval` or lease TTL)

---

## 10. CLI Client

```
zvault-cli — Command-line interface for ZVault

USAGE:
    zvault <COMMAND>

SYSTEM:
    init              Initialize a new vault
    unseal            Submit unseal key shares
    seal              Seal the vault
    status            Show seal status and health

SECRETS (KV):
    kv get <path>                 Read a secret
    kv put <path> key=value ...   Write a secret
    kv delete <path>              Soft-delete a secret
    kv destroy <path> -versions=1,2  Hard-destroy versions
    kv list <prefix>              List secrets under prefix
    kv metadata get <path>        Show version history

DYNAMIC SECRETS:
    database config create <name> ...    Configure a DB connection
    database roles create <name> ...     Create a credential role
    database creds <role>                Generate credentials
    lease renew <lease-id>               Renew a lease
    lease revoke <lease-id>              Revoke a lease

TRANSIT:
    transit keys create <name>           Create an encryption key
    transit encrypt <key> plaintext=...  Encrypt data
    transit decrypt <key> ciphertext=... Decrypt data
    transit rewrap <key> ciphertext=...  Re-encrypt with latest key version
    transit sign <key> input=...         Sign data
    transit verify <key> input=... sig=... Verify signature

PKI:
    pki ca generate-root ...             Generate root CA
    pki ca generate-intermediate ...     Generate intermediate CA
    pki roles create <name> ...          Create a certificate role
    pki issue <role> common_name=...     Issue a certificate
    pki crl rotate                       Rebuild CRL

AUTH:
    auth token create -policies=...      Create a token
    auth token revoke <token>            Revoke a token
    auth service-account create ...      Create a service account
    auth enable oidc ...                 Enable OIDC auth
    auth enable kubernetes ...           Enable K8s auth

POLICY:
    policy write <name> <file>           Create/update a policy
    policy read <name>                   Read a policy
    policy list                          List all policies
    policy delete <name>                 Delete a policy

AUDIT:
    audit enable file path=/var/log/...  Enable file audit backend
    audit list                           List audit backends
    audit query -from=... -to=... ...    Query audit log

OPERATOR:
    server start                         Start the ZVault server
    server start -cluster ...            Start in HA cluster mode
```

---

## 11. Secret Rotation

Automated rotation for secrets that support it.

### 11.1 Rotation Configuration

```json
{
  "path": "secret/data/production/stripe-api-key",
  "rotation_period": "720h",
  "rotation_hook": {
    "type": "http",
    "url": "https://api.stripe.com/v1/api_keys/rotate",
    "method": "POST",
    "headers": { "Authorization": "Bearer {{current_value}}" },
    "response_path": "$.new_key"
  }
}
```

### 11.2 Rotation Flow

```
Scheduler tick
  │
  ├── Check rotation_period against last_rotated
  │
  ├── If due:
  │     1. Call rotation hook (HTTP webhook, script, or built-in)
  │     2. Get new secret value from hook response
  │     3. Write new version to KV engine
  │     4. Audit log the rotation
  │     5. Update last_rotated timestamp
  │
  └── If hook fails:
        1. Retry with exponential backoff (3 attempts)
        2. If all retries fail, emit alert metric + audit entry
        3. Do NOT delete the current secret
```

---

## 12. Observability

### 12.1 Prometheus Metrics

```
# Seal status
zvault_sealed{node="node-1"} 0

# Request metrics
zvault_http_requests_total{method="GET", path="/v1/secret/*", status="200"} 1523
zvault_http_request_duration_seconds{method="GET", path="/v1/secret/*", quantile="0.99"} 0.003

# Engine metrics
zvault_secrets_engine_operations_total{engine="kv", operation="read"} 892
zvault_secrets_engine_operations_total{engine="database", operation="generate"} 45
zvault_secrets_engine_operations_total{engine="transit", operation="encrypt"} 3201

# Lease metrics
zvault_leases_active{engine="database"} 12
zvault_leases_expired_total{engine="database"} 340
zvault_leases_revoked_total{engine="database"} 15

# Raft metrics (HA mode)
zvault_raft_leader{node="node-1"} 1
zvault_raft_peers{node="node-1"} 3
zvault_raft_commit_index{node="node-1"} 48291
zvault_raft_apply_duration_seconds{quantile="0.99"} 0.001

# Storage metrics
zvault_storage_operations_total{backend="rocksdb", operation="get"} 5000
zvault_storage_operations_total{backend="rocksdb", operation="put"} 1200

# Audit metrics
zvault_audit_entries_total 8923
zvault_audit_failures_total 0

# Token metrics
zvault_tokens_active 34
zvault_tokens_expired_total 120

# Rotation metrics
zvault_rotation_success_total 15
zvault_rotation_failure_total 1
```

### 12.2 Structured Logging

All logs are JSON-structured via `tracing` + `tracing-subscriber`:

```json
{
  "timestamp": "2026-02-08T10:30:00Z",
  "level": "INFO",
  "target": "vaultrs::routes::secrets",
  "message": "secret read",
  "path": "secret/data/production/db-password",
  "actor": "deploy-bot",
  "version": 3,
  "duration_ms": 1.2
}
```

---

## 13. Security Hardening

### 13.1 TLS

All client-facing and inter-node communication uses TLS 1.3 via `rustls`
(pure Rust, no OpenSSL dependency). mTLS for inter-node cluster traffic.

### 13.2 Memory Protection

- `zeroize` crate: All key material implements `Zeroize` + `ZeroizeOnDrop`
- `mlock`: Pin key material pages to prevent swapping to disk
- No core dumps: Set `RLIMIT_CORE` to 0 on startup

### 13.3 Rate Limiting

Per-token rate limiting via token bucket algorithm:
- Default: 500 requests/sec per token
- Configurable per policy
- Returns `429 Too Many Requests` with `Retry-After` header

### 13.4 Threat Model

| Threat | Mitigation |
|---|---|
| Storage compromise | All data encrypted via barrier (AES-256-GCM) |
| Memory dump / cold boot | `zeroize` + `mlock` + no core dumps |
| Token theft | SHA-256 hashed in storage; short TTLs; token hierarchy revocation |
| Replay attacks | TLS 1.3; unique nonce per encryption; request IDs |
| Insider threat | Immutable audit log; RBAC; Shamir split (no single person has full unseal key) |
| Compromised node in cluster | Raft requires quorum; each node unsealed independently |
| Side-channel attacks | Constant-time comparison for tokens; no timing oracles |
| Quantum computing | Not addressed (no PQC yet — future consideration) |
| Denial of service | Rate limiting; separate cluster port; resource limits |

---

## 14. Full Dependency Stack

| Crate | Purpose | Why this one |
|---|---|---|
| `axum` 0.8 | HTTP framework | Best async Rust web framework, tower ecosystem |
| `tokio` 1.x | Async runtime | Industry standard |
| `rustls` + `tokio-rustls` | TLS | Pure Rust, no OpenSSL |
| `rocksdb` | Storage backend (default) | Proven at scale, embedded |
| `redb` | Storage backend (alt) | Pure Rust, simpler |
| `openraft` | Raft consensus (HA) | Async, 70k writes/sec, active development |
| `aes-gcm` 0.10 | AES-256-GCM AEAD | RustCrypto, audited |
| `hkdf` + `sha2` | Key derivation (per-engine keys) | Standard, RustCrypto |
| `argon2` 0.5 | KDF for password-based unseal (optional) | Argon2id, memory-hard |
| `sharks` 0.5 | Shamir's Secret Sharing | Simple, correct |
| `ed25519-dalek` 2.x | Ed25519 signing (transit, audit) | Pure Rust, fast |
| `p256` | ECDSA P-256 (transit) | RustCrypto |
| `rsa` | RSA (transit) | RustCrypto |
| `rcgen` | X.509 certificate generation (PKI) | Used by rustls team |
| `x509-parser` | Certificate parsing | Well-maintained |
| `jsonwebtoken` 9.x | JWT validation (OIDC, K8s auth) | Standard |
| `reqwest` | HTTP client (OIDC discovery, webhooks) | Best async HTTP client |
| `kube` + `kube-runtime` | K8s operator | CNCF Sandbox, de facto standard |
| `serde` + `serde_json` | Serialization | Standard |
| `uuid` 1.x | ID generation | Standard |
| `chrono` 0.4 | Timestamps | Standard |
| `tracing` + `tracing-subscriber` | Structured logging | Standard |
| `metrics` + `metrics-exporter-prometheus` | Prometheus metrics | Clean API |
| `zeroize` 1.x | Secure memory clearing | RustCrypto |
| `base64` 0.22 | Encoding | Standard |
| `hex` | Hex encoding | Standard |
| `hmac` 0.12 | HMAC for audit field hashing | RustCrypto |
| `clap` 4.x | CLI argument parsing | Best in class |
| `toml` | Config file parsing | Standard for Rust |
| `thiserror` + `anyhow` | Error handling | Standard |
| `dotenvy` | Env file loading (dev) | Standard |
| `leptos` 0.7 | Reactive UI framework (SSR + CSR) | Full-stack Rust, shared types |
| `leptos_router` 0.7 | Client-side routing | Leptos ecosystem |
| `leptos_axum` 0.7 | Axum SSR integration | Serves UI from same binary |

---

## 15. Project Structure

```
zvault/
├── Cargo.toml                          # workspace root
├── zvault.toml.example                 # example server config
├── docs/
│   └── DESIGN.md                       # ← you are here
│
├── crates/
│   ├── zvault-server/                 # main server binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                 # bootstrap, config loading, server start
│   │       ├── config.rs               # TOML config parsing
│   │       ├── server.rs               # Axum router setup, TLS, middleware
│   │       ├── error.rs                # AppError enum, HTTP error mapping
│   │       └── routes/
│   │           ├── mod.rs
│   │           ├── sys.rs              # /v1/sys/* (init, seal, unseal, health)
│   │           ├── secrets.rs          # /v1/<engine-mount>/* dispatch
│   │           ├── auth.rs             # /v1/auth/* (login, token mgmt)
│   │           ├── policy.rs           # /v1/sys/policy/*
│   │           └── audit.rs            # /v1/sys/audit/*
│   │
│   ├── zvault-core/                   # core library (shared logic)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── seal.rs                 # Shamir init/unseal/seal
│   │       ├── barrier.rs              # encryption barrier (AES-256-GCM)
│   │       ├── crypto.rs               # encrypt/decrypt, key gen, HKDF, zeroize
│   │       ├── lease.rs                # lease manager (create, renew, revoke, tick)
│   │       ├── policy.rs               # policy evaluation engine
│   │       ├── token.rs                # token creation, hierarchy, revocation
│   │       ├── rotation.rs             # secret rotation scheduler
│   │       ├── audit.rs                # audit entry types, HMAC, backends trait
│   │       ├── mount.rs                # engine mount table management
│   │       └── types.rs                # shared types (RequestContext, etc.)
│   │
│   ├── zvault-engines/                # secrets engine implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # SecretsEngine trait + EngineRouter
│   │       ├── kv.rs                   # KV v2 engine
│   │       ├── database/
│   │       │   ├── mod.rs              # database engine core
│   │       │   ├── postgres.rs         # PostgreSQL plugin
│   │       │   └── mysql.rs            # MySQL plugin
│   │       ├── transit.rs              # transit encryption engine
│   │       └── pki.rs                  # PKI / CA engine
│   │
│   ├── zvault-storage/                # storage backend abstraction
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # StorageBackend trait
│   │       ├── rocksdb.rs              # RocksDB implementation
│   │       ├── redb.rs                 # redb implementation (feature-gated)
│   │       └── memory.rs               # in-memory (for testing)
│   │
│   ├── zvault-raft/                   # Raft consensus layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── network.rs              # inter-node gRPC/TCP transport
│   │       ├── state_machine.rs        # Raft state machine (applies writes to storage)
│   │       └── snapshot.rs             # Raft snapshot management
│   │
│   ├── zvault-auth/                   # auth method implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # AuthMethod trait
│   │       ├── token.rs                # token auth
│   │       ├── oidc.rs                 # OIDC/OAuth2 auth
│   │       ├── kubernetes.rs           # K8s service account auth
│   │       └── approle.rs              # AppRole auth
│   │
│   ├── zvault-cli/                    # CLI client binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── client.rs               # HTTP client wrapper
│   │       └── commands/
│   │           ├── mod.rs
│   │           ├── sys.rs              # init, unseal, seal, status
│   │           ├── kv.rs               # kv get/put/delete/list
│   │           ├── database.rs         # database config/roles/creds
│   │           ├── transit.rs          # encrypt/decrypt/sign/verify
│   │           ├── pki.rs              # ca/roles/issue/crl
│   │           ├── auth.rs             # token/service-account/enable
│   │           ├── policy.rs           # policy CRUD
│   │           └── audit.rs            # audit enable/query
│   │
│   ├── zvault-operator/              # Kubernetes operator binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── controller.rs           # reconciliation loop
│   │       ├── crd.rs                  # ZVaultSecret CRD definition
│   │       └── client.rs              # ZVault API client
│   │
│   ├── zvault-types/                 # shared request/response types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # re-exports
│   │       ├── sys.rs                  # InitRequest, SealStatus, Health, etc.
│   │       ├── secrets.rs              # KV, database, transit, PKI types
│   │       ├── auth.rs                 # token, OIDC, AppRole types
│   │       └── policy.rs              # Policy, Rule types
│   │
│   └── zvault-ui/                    # Leptos web UI (WASM + SSR)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # app root, router
│           ├── pages/
│           │   ├── mod.rs
│           │   ├── dashboard.rs        # seal status, cluster health, metrics
│           │   ├── secrets.rs          # secret browser (tree view, CRUD)
│           │   ├── init.rs             # vault initialization wizard
│           │   ├── unseal.rs           # unseal share submission
│           │   ├── policies.rs         # policy editor
│           │   ├── audit.rs            # audit log viewer with filters
│           │   └── leases.rs           # lease management table
│           └── components/
│               ├── mod.rs
│               ├── nav.rs              # sidebar navigation
│               ├── status_badge.rs     # sealed/unsealed/standby indicator
│               └── secret_tree.rs      # hierarchical secret browser
│
├── deploy/
│   ├── docker/
│   │   └── Dockerfile                 # multi-stage build
│   └── helm/
│       └── zvault/                   # Helm chart
│           ├── Chart.yaml
│           ├── values.yaml
│           └── templates/
│               ├── deployment.yaml
│               ├── service.yaml
│               ├── statefulset.yaml   # for HA mode
│               └── operator.yaml
│
└── tests/
    ├── integration/
    │   ├── init_unseal.rs
    │   ├── kv_engine.rs
    │   ├── database_engine.rs
    │   ├── transit_engine.rs
    │   ├── pki_engine.rs
    │   ├── auth_policies.rs
    │   ├── lease_lifecycle.rs
    │   ├── rotation.rs
    │   └── cluster_ha.rs
    └── fixtures/
        └── policies/
```

---

## 16. API Reference

### 16.1 System

```
POST   /v1/sys/init                    Initialize vault
POST   /v1/sys/unseal                  Submit unseal shares
POST   /v1/sys/seal                    Seal the vault
GET    /v1/sys/seal-status             Seal status
GET    /v1/sys/health                  Health check (200/429/500/503)
GET    /v1/sys/leader                  HA leader info
POST   /v1/sys/mounts/<path>          Mount a secrets engine
GET    /v1/sys/mounts                  List mounted engines
DELETE /v1/sys/mounts/<path>          Unmount an engine
POST   /v1/sys/policy/<name>          Create/update policy
GET    /v1/sys/policy/<name>          Read policy
GET    /v1/sys/policy                  List policies
DELETE /v1/sys/policy/<name>          Delete policy
POST   /v1/sys/audit/<name>           Enable audit backend
GET    /v1/sys/audit                   List audit backends
DELETE /v1/sys/audit/<name>           Disable audit backend
GET    /v1/sys/audit/query             Query audit log
POST   /v1/sys/leases/renew           Renew a lease
POST   /v1/sys/leases/revoke          Revoke a lease
GET    /v1/sys/metrics                 Prometheus metrics
```

### 16.2 KV Engine (mounted at /v1/secret/)

```
GET    /v1/secret/data/<path>          Read secret (latest or ?version=N)
POST   /v1/secret/data/<path>          Write secret
DELETE /v1/secret/data/<path>          Soft-delete latest version
POST   /v1/secret/delete/<path>        Soft-delete specific versions
POST   /v1/secret/undelete/<path>      Undelete versions
POST   /v1/secret/destroy/<path>       Hard-destroy versions
GET    /v1/secret/metadata/<path>      Read secret metadata
LIST   /v1/secret/metadata/<prefix>    List secrets
DELETE /v1/secret/metadata/<path>      Delete all versions + metadata
```

### 16.3 Database Engine (mounted at /v1/database/)

```
POST   /v1/database/config/<name>      Configure DB connection
GET    /v1/database/config/<name>      Read DB config
DELETE /v1/database/config/<name>      Delete DB config
POST   /v1/database/roles/<name>       Create/update role
GET    /v1/database/roles/<name>       Read role
DELETE /v1/database/roles/<name>       Delete role
GET    /v1/database/creds/<role>       Generate credentials
POST   /v1/database/rotate-root/<name> Rotate root credentials
```

### 16.4 Transit Engine (mounted at /v1/transit/)

```
POST   /v1/transit/keys/<name>         Create encryption key
GET    /v1/transit/keys/<name>         Read key info (no key material)
POST   /v1/transit/keys/<name>/rotate  Rotate key
POST   /v1/transit/encrypt/<name>      Encrypt plaintext
POST   /v1/transit/decrypt/<name>      Decrypt ciphertext
POST   /v1/transit/rewrap/<name>       Re-encrypt with latest key version
POST   /v1/transit/sign/<name>         Sign data
POST   /v1/transit/verify/<name>       Verify signature
POST   /v1/transit/hash                Hash data
POST   /v1/transit/random              Generate random bytes
POST   /v1/transit/datakey/<name>      Generate data encryption key
```

### 16.5 PKI Engine (mounted at /v1/pki/)

```
POST   /v1/pki/root/generate           Generate root CA
POST   /v1/pki/intermediate/generate   Generate intermediate CSR
POST   /v1/pki/intermediate/set-signed Set signed intermediate cert
POST   /v1/pki/roles/<name>            Create/update role
GET    /v1/pki/roles/<name>            Read role
POST   /v1/pki/issue/<role>            Issue certificate
POST   /v1/pki/sign/<role>             Sign CSR
POST   /v1/pki/revoke                  Revoke certificate
GET    /v1/pki/crl                     Get CRL (DER)
GET    /v1/pki/crl/pem                 Get CRL (PEM)
GET    /v1/pki/ca                      Get CA cert
GET    /v1/pki/ca/chain                Get CA chain
POST   /v1/pki/tidy                    Tidy up expired certs
```

### 16.6 Auth

```
POST   /v1/auth/token/create           Create token
POST   /v1/auth/token/renew            Renew token
POST   /v1/auth/token/revoke           Revoke token
GET    /v1/auth/token/lookup            Lookup token info
POST   /v1/auth/approle/login          AppRole login
POST   /v1/auth/oidc/login             OIDC login
POST   /v1/auth/kubernetes/login       K8s login
```

---

## 17. Web UI (Leptos)

Full-stack Rust UI built with Leptos, served from the same binary as the API.

### 17.1 Why Leptos

- **Shared types**: Request/response types defined once in `zvault-types`,
  used by server, CLI, and UI. No OpenAPI codegen, no drift.
- **Single binary**: The server serves both the API at `/v1/*` and the UI at
  `/`. No separate frontend deployment.
- **SSR + WASM hydration**: Works without WASM if needed (locked-down
  environments), full interactivity when WASM loads.
- **Same toolchain**: One language, one build system, one CI pipeline.

### 17.2 UI Pages

| Page | Route | Purpose |
|---|---|---|
| Dashboard | `/` | Seal status, cluster health, active leases, recent audit entries |
| Init | `/init` | Vault initialization wizard (share count, threshold) |
| Unseal | `/unseal` | Submit unseal shares one at a time |
| Secrets | `/secrets/*` | Hierarchical secret browser with tree view, read/write/delete |
| Policies | `/policies` | Policy list, JSON editor, create/delete |
| Audit | `/audit` | Audit log table with date range, path, and actor filters |
| Leases | `/leases` | Active lease table with renew/revoke actions |
| Auth | `/auth` | Auth method configuration, token management |

### 17.3 Architecture

```
zvault-server binary
  ├── /v1/*          → Axum API handlers
  └── /*             → Leptos SSR + static WASM/JS/CSS assets

zvault-types crate (shared)
  ├── Used by zvault-server (serialize responses)
  ├── Used by zvault-ui (deserialize responses)
  └── Used by zvault-cli (deserialize responses)
```

The `zvault-types` crate contains all API request/response structs with
`serde::Serialize + Deserialize`. Both the server and UI depend on it,
ensuring type-level API compatibility at compile time.

### 17.4 Dependencies

| Crate | Purpose |
|---|---|
| `leptos` 0.7 | Reactive UI framework (SSR + CSR) |
| `leptos_router` 0.7 | Client-side routing |
| `leptos_axum` 0.7 | Axum integration for SSR |
| `zvault-types` | Shared API types |

---

## 18. Implementation Order

Not phases. Just the order we build things, each step produces working software.

| # | Milestone | What's working after this step |
|---|---|---|
| 1 | **Workspace + storage trait** | Cargo workspace, StorageBackend trait, RocksDB + memory impls |
| 2 | **Crypto + barrier** | AES-256-GCM encrypt/decrypt, HKDF, zeroize wrappers, barrier layer |
| 3 | **Seal/unseal** | Shamir init, unseal, seal. Root key lifecycle. |
| 4 | **Server skeleton** | Axum server, TLS, config loading, /sys/init, /sys/unseal, /sys/health |
| 5 | **Token auth + policies** | Token creation, hash storage, policy evaluation, auth middleware |
| 6 | **KV engine** | Full KV v2: read, write, delete, versions, metadata, list |
| 7 | **Audit system** | Audit trait, file backend, HMAC'd fields, fail-closed |
| 8 | **Lease manager** | Lease create/renew/revoke/tick, background task |
| 9 | **Database engine** | Postgres + MySQL dynamic credential generation |
| 10 | **Transit engine** | Encrypt/decrypt/sign/verify/rewrap/datakey |
| 11 | **PKI engine** | Root CA, intermediate CA, cert issuance, CRL |
| 12 | **CLI client** | Full CLI covering all operations |
| 13 | **OIDC + AppRole + K8s auth** | All auth methods working |
| 14 | **Secret rotation** | Rotation scheduler, webhook hooks |
| 15 | **Raft HA** | Multi-node cluster, leader election, replication |
| 16 | **Prometheus metrics** | Full metrics endpoint |
| 17 | **K8s operator** | CRD, controller, reconciliation loop |
| 18 | **Web UI (Leptos)** | Dashboard, secret browser, policy editor, audit viewer |
| 19 | **Docker + Helm** | Container image, Helm chart for deployment |

---

## 19. Open Questions

Things to decide before or during implementation:

1. **Auto-unseal via cloud KMS?** Vault supports auto-unseal using AWS KMS,
   GCP KMS, or Azure Key Vault instead of Shamir shares. Do we want this?
   (Probably yes, as an alternative to Shamir.)

2. **gRPC vs HTTP for inter-node Raft transport?** gRPC is more efficient
   but adds a `tonic`/`prost` dependency. Raw TCP with length-prefixed
   frames is simpler. Vault uses its own TCP protocol.

3. **Secret path encryption?** Currently paths are stored in plaintext
   (needed for prefix queries). We could encrypt paths with a deterministic
   scheme (SIV mode) to hide path names from storage, at the cost of
   no prefix listing without a separate index.

4. **Namespace / multi-tenancy?** Vault Enterprise has namespaces for
   multi-tenant isolation. Do we want this in v1?

---

*This document is the complete design. No features are deferred. Let's build it.*
