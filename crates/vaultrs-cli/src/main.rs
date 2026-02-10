//! `ZVault` CLI — command-line client for the `ZVault` secrets manager.
//!
//! A standalone HTTP client that communicates with the `ZVault` server.
//! No internal crate dependencies — talks exclusively via the REST API.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde_json::Value;

// ── ANSI color helpers ───────────────────────────────────────────────

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";
const BG_RED: &str = "\x1b[41m";
const BG_GREEN: &str = "\x1b[42m";

// ── ASCII banner ─────────────────────────────────────────────────────

const BANNER: &str = r#"
                 ╔═╗╦  ╦┌─┐┬ ┬┬ ┌┬┐
                 ╔═╝╚╗╔╝├─┤│ ││  │
                 ╚═╝ ╚╝ ┴ ┴└─┘┴─┘┴
"#;

const BANNER_SMALL: &str = "⟐ ZVault";

fn print_banner() {
    println!("{CYAN}{BOLD}{BANNER}{RESET}");
    println!(
        "  {DIM}Secrets management, done right.{RESET}"
    );
    println!(
        "  {DIM}AES-256-GCM • Shamir's Secret Sharing • Zero-Trust{RESET}"
    );
    println!();
}

// ── CLI structure ────────────────────────────────────────────────────

/// ZVault — secrets management, done right.
#[derive(Parser)]
#[command(
    name = "zvault",
    version,
    about = "ZVault CLI — manage secrets, tokens, policies, and transit keys",
    long_about = None,
    after_help = format!(
        "{DIM}Environment variables:{RESET}\n  \
         VAULT_ADDR    Server address (default: http://127.0.0.1:8200)\n  \
         VAULT_TOKEN   Authentication token\n\n\
         {DIM}Examples:{RESET}\n  \
         zvault status\n  \
         zvault init --shares 5 --threshold 3\n  \
         zvault kv put myapp/config db_host=10.0.0.1 db_port=5432\n  \
         zvault transit encrypt my-key $(echo -n 'hello' | base64)"
    ),
)]
struct Cli {
    /// ZVault server address.
    #[arg(long, env = "VAULT_ADDR", default_value = "http://127.0.0.1:8200")]
    addr: String,

    /// Authentication token.
    #[arg(long, env = "VAULT_TOKEN")]
    token: Option<String>,

    /// Disable colored output.
    #[arg(long, default_value = "false")]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show vault seal status and health.
    Status,
    /// Initialize a new vault with Shamir's Secret Sharing.
    Init {
        /// Number of unseal key shares to generate (1-10).
        #[arg(long, default_value = "5")]
        shares: u8,
        /// Minimum shares required to unseal (2..=shares).
        #[arg(long, default_value = "3")]
        threshold: u8,
    },
    /// Submit an unseal key share.
    Unseal {
        /// Base64-encoded unseal key share.
        #[arg(long)]
        share: String,
    },
    /// Seal the vault (zeroizes all key material).
    Seal,
    /// Token authentication operations.
    Token {
        #[command(subcommand)]
        action: TokenCommands,
    },
    /// KV v2 secrets engine operations.
    Kv {
        #[command(subcommand)]
        action: KvCommands,
    },
    /// Access control policy operations.
    Policy {
        #[command(subcommand)]
        action: PolicyCommands,
    },
    /// Transit encryption-as-a-service operations.
    Transit {
        #[command(subcommand)]
        action: TransitCommands,
    },
    /// Database secrets engine operations.
    Database {
        #[command(subcommand)]
        action: DatabaseCommands,
    },
    /// PKI certificate authority operations.
    Pki {
        #[command(subcommand)]
        action: PkiCommands,
    },
    /// AppRole authentication operations.
    Approle {
        #[command(subcommand)]
        action: AppRoleCommands,
    },
}

#[derive(Subcommand)]
enum TokenCommands {
    /// Create a new child token.
    Create {
        /// Comma-separated policies to attach.
        #[arg(long, value_delimiter = ',')]
        policies: Option<Vec<String>>,
        /// Time-to-live (e.g., "1h", "30m", "3600s").
        #[arg(long)]
        ttl: Option<String>,
    },
    /// Look up the current token's metadata.
    Lookup,
}

#[derive(Subcommand)]
enum KvCommands {
    /// Write a secret (key=value pairs).
    Put {
        /// Secret path (e.g., "myapp/config").
        path: String,
        /// Key-value pairs in key=value format.
        #[arg(required = true)]
        data: Vec<String>,
    },
    /// Read a secret by path.
    Get {
        /// Secret path.
        path: String,
    },
    /// Soft-delete a secret.
    Delete {
        /// Secret path.
        path: String,
    },
    /// List secret keys under a prefix.
    List {
        /// Path prefix.
        path: String,
    },
}

#[derive(Subcommand)]
enum PolicyCommands {
    /// Create or update a policy from a JSON file.
    Write {
        /// Policy name.
        name: String,
        /// Path to JSON policy file.
        file: String,
    },
    /// Read a policy by name.
    Read {
        /// Policy name.
        name: String,
    },
    /// List all policy names.
    List,
    /// Delete a policy.
    Delete {
        /// Policy name.
        name: String,
    },
}

#[derive(Subcommand)]
enum TransitCommands {
    /// Create a new named encryption key.
    CreateKey {
        /// Key name.
        name: String,
    },
    /// Rotate a named key to a new version.
    RotateKey {
        /// Key name.
        name: String,
    },
    /// Encrypt base64-encoded plaintext.
    Encrypt {
        /// Key name.
        key: String,
        /// Base64-encoded plaintext.
        plaintext: String,
    },
    /// Decrypt ciphertext (vault:vN:base64 format).
    Decrypt {
        /// Key name.
        key: String,
        /// Ciphertext string.
        ciphertext: String,
    },
    /// List all transit key names.
    ListKeys,
    /// Show metadata for a named key.
    KeyInfo {
        /// Key name.
        name: String,
    },
}

#[derive(Subcommand)]
enum DatabaseCommands {
    /// Configure a database connection.
    Configure {
        /// Connection name.
        name: String,
        /// Database plugin ("postgresql" or "mysql").
        #[arg(long)]
        plugin: String,
        /// Connection URL.
        #[arg(long)]
        connection_url: String,
    },
    /// Create a database role.
    CreateRole {
        /// Role name.
        name: String,
        /// Database connection name.
        #[arg(long)]
        db_name: String,
        /// SQL creation statement.
        #[arg(long)]
        creation_statement: String,
    },
    /// Generate dynamic credentials for a role.
    Creds {
        /// Role name.
        name: String,
    },
    /// List all database roles.
    ListRoles,
    /// List all database configs.
    ListConfigs,
}

#[derive(Subcommand)]
enum PkiCommands {
    /// Generate a self-signed root CA.
    GenerateRoot {
        /// CA common name.
        #[arg(long)]
        common_name: String,
        /// Validity in hours (default: 87600 = 10 years).
        #[arg(long, default_value = "87600")]
        ttl_hours: u64,
    },
    /// Issue a certificate using a role.
    Issue {
        /// Role name.
        role: String,
        /// Certificate common name (domain).
        #[arg(long)]
        common_name: String,
        /// TTL in hours.
        #[arg(long)]
        ttl_hours: Option<u64>,
    },
    /// List all PKI roles.
    ListRoles,
    /// List all issued certificates.
    ListCerts,
    /// Create a PKI role.
    CreateRole {
        /// Role name.
        name: String,
        /// Comma-separated allowed domains.
        #[arg(long, value_delimiter = ',')]
        allowed_domains: Vec<String>,
        /// Allow subdomains.
        #[arg(long, default_value = "false")]
        allow_subdomains: bool,
    },
}

#[derive(Subcommand)]
enum AppRoleCommands {
    /// Create an AppRole role.
    CreateRole {
        /// Role name.
        name: String,
        /// Comma-separated policies.
        #[arg(long, value_delimiter = ',')]
        policies: Vec<String>,
    },
    /// Get the role ID for a named role.
    RoleId {
        /// Role name.
        name: String,
    },
    /// Generate a secret ID for a role.
    SecretId {
        /// Role name.
        name: String,
    },
    /// Login with role_id and secret_id.
    Login {
        /// Role ID.
        #[arg(long)]
        role_id: String,
        /// Secret ID.
        #[arg(long)]
        secret_id: String,
    },
    /// List all AppRole roles.
    ListRoles,
}

// ── Pretty output helpers ────────────────────────────────────────────

fn header(icon: &str, title: &str) {
    println!("{BOLD}{CYAN}{icon} {title}{RESET}");
    println!("{DIM}─────────────────────────────────────────{RESET}");
}

fn kv_line(key: &str, value: &str) {
    println!("  {DIM}{key:<20}{RESET} {WHITE}{value}{RESET}");
}

fn success(msg: &str) {
    println!("{GREEN}{BOLD}✓{RESET} {msg}");
}

fn warning(msg: &str) {
    println!("{YELLOW}{BOLD}⚠{RESET} {YELLOW}{msg}{RESET}");
}

fn print_seal_status(resp: &Value) {
    let initialized = resp.get("initialized").and_then(Value::as_bool).unwrap_or(false);
    let sealed = resp.get("sealed").and_then(Value::as_bool).unwrap_or(true);
    let threshold = resp.get("threshold").and_then(Value::as_u64).unwrap_or(0);
    let shares = resp.get("shares").and_then(Value::as_u64).unwrap_or(0);
    let progress = resp.get("progress").and_then(Value::as_u64).unwrap_or(0);

    header("🔐", "Vault Status");

    let init_status = if initialized {
        format!("{GREEN}yes{RESET}")
    } else {
        format!("{RED}no{RESET}")
    };
    kv_line("Initialized", &init_status);

    let seal_status = if sealed {
        format!("{BG_RED}{WHITE}{BOLD} SEALED {RESET}")
    } else {
        format!("{BG_GREEN}{WHITE}{BOLD} UNSEALED {RESET}")
    };
    kv_line("Seal Status", &seal_status);

    if shares > 0 {
        kv_line("Total Shares", &shares.to_string());
        kv_line("Threshold", &threshold.to_string());
    }

    if sealed && progress > 0 {
        let bar = progress_bar(progress, threshold);
        kv_line("Unseal Progress", &format!("{bar} {progress}/{threshold}"));
    }

    println!();
}

fn progress_bar(current: u64, total: u64) -> String {
    let width = 20;
    let filled = if total > 0 {
        ((current * width) / total) as usize
    } else {
        0
    };
    let empty = width as usize - filled;
    format!(
        "{CYAN}[{}{DIM}{}]{RESET}",
        "█".repeat(filled),
        "░".repeat(empty)
    )
}

fn print_init_response(resp: &Value) {
    print_banner();
    header("🔑", "Vault Initialized");
    println!();

    if let Some(shares) = resp.get("unseal_shares").and_then(Value::as_array) {
        println!(
            "  {YELLOW}{BOLD}⚠  Store these unseal keys in separate secure locations!{RESET}"
        );
        println!(
            "  {YELLOW}   They will NOT be shown again.{RESET}"
        );
        println!();

        for (i, share) in shares.iter().enumerate() {
            if let Some(s) = share.as_str() {
                let num = i.checked_add(1).unwrap_or(i);
                println!("  {DIM}Unseal Key {num}:{RESET}  {MAGENTA}{s}{RESET}");
            }
        }
    }

    println!();

    if let Some(token) = resp.get("root_token").and_then(Value::as_str) {
        println!(
            "  {DIM}Root Token:{RESET}    {GREEN}{BOLD}{token}{RESET}"
        );
    }

    println!();
    println!(
        "  {DIM}Vault is initialized but {YELLOW}{BOLD}sealed{RESET}{DIM}. Use `zvault unseal`{RESET}"
    );
    println!(
        "  {DIM}with the required threshold of key shares to unseal.{RESET}"
    );
    println!();
}

fn print_unseal_response(resp: &Value) {
    let sealed = resp.get("sealed").and_then(Value::as_bool).unwrap_or(true);
    let threshold = resp.get("threshold").and_then(Value::as_u64).unwrap_or(0);
    let progress = resp.get("progress").and_then(Value::as_u64).unwrap_or(0);

    if sealed {
        header("🔓", "Unseal Progress");
        let bar = progress_bar(progress, threshold);
        println!("  {bar} {BOLD}{progress}{RESET}/{threshold} shares submitted");
        println!();
        let remaining = threshold.saturating_sub(progress);
        println!(
            "  {DIM}{remaining} more share(s) needed to unseal.{RESET}"
        );
    } else {
        println!();
        println!(
            "  {BG_GREEN}{WHITE}{BOLD} ✓ VAULT UNSEALED {RESET}"
        );
        println!();
        println!(
            "  {DIM}The vault is now ready to accept requests.{RESET}"
        );
    }
    println!();
}

fn print_token_response(resp: &Value) {
    header("🪙", "Token Created");

    if let Some(token) = resp.get("client_token").and_then(Value::as_str) {
        println!();
        println!("  {DIM}Token:{RESET}       {GREEN}{BOLD}{token}{RESET}");
    }

    if let Some(policies) = resp.get("policies").and_then(Value::as_array) {
        let names: Vec<&str> = policies.iter().filter_map(Value::as_str).collect();
        kv_line("Policies", &names.join(", "));
    }

    if let Some(renewable) = resp.get("renewable").and_then(Value::as_bool) {
        kv_line("Renewable", if renewable { "yes" } else { "no" });
    }

    if let Some(dur) = resp.get("lease_duration").and_then(Value::as_i64) {
        kv_line("TTL", &format_duration(dur));
    }

    println!();
}

fn print_token_lookup(resp: &Value) {
    header("🪙", "Token Lookup");

    if let Some(hash) = resp.get("token_hash").and_then(Value::as_str) {
        let short = if hash.len() > 12 { &hash[..12] } else { hash };
        kv_line("Token Hash", &format!("{short}..."));
    }

    if let Some(policies) = resp.get("policies").and_then(Value::as_array) {
        let names: Vec<&str> = policies.iter().filter_map(Value::as_str).collect();
        kv_line("Policies", &names.join(", "));
    }

    if let Some(name) = resp.get("display_name").and_then(Value::as_str) {
        kv_line("Display Name", name);
    }

    if let Some(expires) = resp.get("expires_at").and_then(Value::as_str) {
        if expires.is_empty() {
            kv_line("Expires", "never");
        } else {
            kv_line("Expires", expires);
        }
    }

    println!();
}

fn print_secret_response(path: &str, resp: &Value) {
    header("📦", &format!("Secret: {path}"));

    if let Some(data) = resp.get("data") {
        if let Some(obj) = data.as_object() {
            for (k, v) in obj {
                let display = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                kv_line(k, &display);
            }
        } else {
            print_json(data);
        }
    } else {
        print_json(resp);
    }

    if let Some(lease) = resp.get("lease_id").and_then(Value::as_str) {
        if !lease.is_empty() {
            println!();
            kv_line("Lease ID", lease);
        }
    }

    println!();
}

fn print_list_response(path: &str, resp: &Value) {
    header("📂", &format!("Keys: {path}"));

    if let Some(data) = resp.get("data") {
        if let Some(keys) = data.get("keys").and_then(Value::as_array) {
            if keys.is_empty() {
                println!("  {DIM}(empty){RESET}");
            } else {
                for key in keys {
                    if let Some(k) = key.as_str() {
                        println!("  {CYAN}├─{RESET} {k}");
                    }
                }
            }
        } else {
            print_json(data);
        }
    } else {
        print_json(resp);
    }

    println!();
}

fn print_policy_list(resp: &Value) {
    header("📜", "Policies");

    if let Some(policies) = resp.get("policies").and_then(Value::as_array) {
        if policies.is_empty() {
            println!("  {DIM}(no policies){RESET}");
        } else {
            for p in policies {
                if let Some(name) = p.as_str() {
                    let icon = match name {
                        "root" => "👑",
                        "default" => "📋",
                        _ => "📜",
                    };
                    println!("  {icon} {name}");
                }
            }
        }
    } else {
        print_json(resp);
    }

    println!();
}

fn print_policy_detail(name: &str, resp: &Value) {
    header("📜", &format!("Policy: {name}"));

    if let Some(rules) = resp.get("rules").and_then(Value::as_array) {
        for rule in rules {
            let path = rule.get("path").and_then(Value::as_str).unwrap_or("?");
            let caps = rule
                .get("capabilities")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            println!("  {CYAN}{path}{RESET}");
            println!("    {DIM}capabilities:{RESET} {caps}");
        }
    } else {
        print_json(resp);
    }

    println!();
}

fn print_transit_key_list(resp: &Value) {
    header("🔐", "Transit Keys");

    if let Some(keys) = resp.get("keys").and_then(Value::as_array) {
        if keys.is_empty() {
            println!("  {DIM}(no keys){RESET}");
        } else {
            for k in keys {
                if let Some(name) = k.as_str() {
                    println!("  {MAGENTA}⚷{RESET}  {name}");
                }
            }
        }
    } else {
        print_json(resp);
    }

    println!();
}

fn print_transit_key_info(resp: &Value) {
    let name = resp.get("name").and_then(Value::as_str).unwrap_or("?");
    header("⚷", &format!("Transit Key: {name}"));

    if let Some(ver) = resp.get("latest_version").and_then(Value::as_u64) {
        kv_line("Latest Version", &format!("v{ver}"));
    }
    if let Some(min) = resp.get("min_decryption_version").and_then(Value::as_u64) {
        kv_line("Min Decrypt Ver", &format!("v{min}"));
    }
    if let Some(enc) = resp.get("supports_encryption").and_then(Value::as_bool) {
        kv_line("Encryption", if enc { "yes" } else { "no" });
    }
    if let Some(dec) = resp.get("supports_decryption").and_then(Value::as_bool) {
        kv_line("Decryption", if dec { "yes" } else { "no" });
    }
    if let Some(count) = resp.get("version_count").and_then(Value::as_u64) {
        kv_line("Versions", &count.to_string());
    }
    if let Some(created) = resp.get("created_at").and_then(Value::as_str) {
        kv_line("Created", created);
    }

    println!();
}

fn print_encrypt_response(resp: &Value) {
    header("🔒", "Encrypted");

    if let Some(ct) = resp.get("ciphertext").and_then(Value::as_str) {
        println!();
        println!("  {DIM}Ciphertext:{RESET}");
        println!("  {MAGENTA}{ct}{RESET}");
    } else {
        print_json(resp);
    }

    println!();
}

fn print_decrypt_response(resp: &Value) {
    header("🔓", "Decrypted");

    if let Some(pt) = resp.get("plaintext").and_then(Value::as_str) {
        println!();
        println!("  {DIM}Plaintext (base64):{RESET}");
        println!("  {GREEN}{pt}{RESET}");
    } else {
        print_json(resp);
    }

    println!();
}

fn format_duration(secs: i64) -> String {
    if secs <= 0 {
        return "none".to_owned();
    }
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if hours > 0 {
        format!("{hours}h{mins}m{s}s")
    } else if mins > 0 {
        format!("{mins}m{s}s")
    } else {
        format!("{s}s")
    }
}

// ── HTTP client ──────────────────────────────────────────────────────

struct Client {
    http: reqwest::Client,
    addr: String,
    token: Option<String>,
}

impl Client {
    fn new(addr: String, token: Option<String>) -> Self {
        let http = reqwest::Client::new();
        Self { http, addr, token }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.addr)
    }

    fn auth_header(&self) -> Result<String> {
        self.token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no token provided — set VAULT_TOKEN or use --token"))
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let token = self.auth_header()?;
        let resp = self
            .http
            .get(self.url(path))
            .header("X-Vault-Token", &token)
            .send()
            .await
            .context("request failed")?;
        handle_response(resp).await
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let token = self.auth_header()?;
        let resp = self
            .http
            .post(self.url(path))
            .header("X-Vault-Token", &token)
            .json(body)
            .send()
            .await
            .context("request failed")?;
        handle_response(resp).await
    }

    async fn post_no_auth(&self, path: &str, body: &Value) -> Result<Value> {
        let resp = self
            .http
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .context("request failed")?;
        handle_response(resp).await
    }

    async fn post_no_body(&self, path: &str) -> Result<Value> {
        let token = self.auth_header()?;
        let resp = self
            .http
            .post(self.url(path))
            .header("X-Vault-Token", &token)
            .send()
            .await
            .context("request failed")?;
        handle_response(resp).await
    }

    async fn delete(&self, path: &str) -> Result<Value> {
        let token = self.auth_header()?;
        let resp = self
            .http
            .delete(self.url(path))
            .header("X-Vault-Token", &token)
            .send()
            .await
            .context("request failed")?;
        handle_response(resp).await
    }

    async fn get_no_auth(&self, path: &str) -> Result<Value> {
        let resp = self
            .http
            .get(self.url(path))
            .send()
            .await
            .context("request failed")?;
        handle_response(resp).await
    }
}

async fn handle_response(resp: reqwest::Response) -> Result<Value> {
    let status = resp.status();
    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    let body = resp.text().await.context("failed to read response body")?;
    if !status.is_success() {
        bail!("server returned {status}: {body}");
    }
    if body.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&body).context("failed to parse response JSON")
}

// ── Command dispatch ─────────────────────────────────────────────────

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let client = Client::new(cli.addr, cli.token);

    match run(client, cli.command).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!();
            eprintln!("  {RED}{BOLD}✗ Error:{RESET} {e:#}");
            eprintln!();
            ExitCode::FAILURE
        }
    }
}

async fn run(client: Client, cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Status => cmd_status(&client).await,
        Commands::Init { shares, threshold } => cmd_init(&client, shares, threshold).await,
        Commands::Unseal { share } => cmd_unseal(&client, &share).await,
        Commands::Seal => cmd_seal(&client).await,
        Commands::Token { action } => cmd_token(&client, action).await,
        Commands::Kv { action } => cmd_kv(&client, action).await,
        Commands::Policy { action } => cmd_policy(&client, action).await,
        Commands::Transit { action } => cmd_transit(&client, action).await,
        Commands::Database { action } => cmd_database(&client, action).await,
        Commands::Pki { action } => cmd_pki(&client, action).await,
        Commands::Approle { action } => cmd_approle(&client, action).await,
    }
}

// ── System commands ──────────────────────────────────────────────────

async fn cmd_status(client: &Client) -> Result<()> {
    println!();
    println!("  {BANNER_SMALL} {DIM}checking health...{RESET}");
    println!();
    let resp = client.get_no_auth("/v1/sys/health").await?;
    print_seal_status(&resp);
    Ok(())
}

async fn cmd_init(client: &Client, shares: u8, threshold: u8) -> Result<()> {
    let body = serde_json::json!({ "shares": shares, "threshold": threshold });
    let resp = client.post_no_auth("/v1/sys/init", &body).await?;
    print_init_response(&resp);
    Ok(())
}

async fn cmd_unseal(client: &Client, share: &str) -> Result<()> {
    let body = serde_json::json!({ "share": share });
    let resp = client.post_no_auth("/v1/sys/unseal", &body).await?;
    print_unseal_response(&resp);
    Ok(())
}

async fn cmd_seal(client: &Client) -> Result<()> {
    client.post_no_body("/v1/sys/seal").await?;
    println!();
    warning("Vault sealed — all key material zeroized from memory.");
    println!();
    Ok(())
}

// ── Token commands ───────────────────────────────────────────────────

async fn cmd_token(client: &Client, action: TokenCommands) -> Result<()> {
    match action {
        TokenCommands::Create { policies, ttl } => {
            let mut body = serde_json::Map::new();
            if let Some(p) = policies {
                body.insert("policies".to_owned(), serde_json::json!(p));
            }
            if let Some(t) = ttl {
                body.insert("ttl".to_owned(), serde_json::json!(t));
            }
            let resp = client
                .post("/v1/auth/token/create", &Value::Object(body))
                .await?;
            println!();
            print_token_response(&resp);
        }
        TokenCommands::Lookup => {
            let resp = client
                .post("/v1/auth/token/lookup-self", &serde_json::json!({}))
                .await?;
            println!();
            print_token_lookup(&resp);
        }
    }
    Ok(())
}

// ── KV commands ──────────────────────────────────────────────────────

async fn cmd_kv(client: &Client, action: KvCommands) -> Result<()> {
    match action {
        KvCommands::Put { path, data } => {
            let map = parse_kv_pairs(&data)?;
            let body = serde_json::json!({ "data": map });
            client
                .post(&format!("/v1/secret/data/{path}"), &body)
                .await?;
            println!();
            success(&format!("Secret written to {BOLD}{path}{RESET}"));
            println!();
        }
        KvCommands::Get { path } => {
            let resp = client.get(&format!("/v1/secret/data/{path}")).await?;
            println!();
            print_secret_response(&path, &resp);
        }
        KvCommands::Delete { path } => {
            client.delete(&format!("/v1/secret/data/{path}")).await?;
            println!();
            success(&format!("Secret at {BOLD}{path}{RESET} deleted."));
            println!();
        }
        KvCommands::List { path } => {
            let resp = client.get(&format!("/v1/secret/list/{path}")).await?;
            println!();
            print_list_response(&path, &resp);
        }
    }
    Ok(())
}

// ── Policy commands ──────────────────────────────────────────────────

async fn cmd_policy(client: &Client, action: PolicyCommands) -> Result<()> {
    match action {
        PolicyCommands::Write { name, file } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("failed to read policy file: {file}"))?;
            let body: Value =
                serde_json::from_str(&content).context("policy file is not valid JSON")?;
            client
                .post(&format!("/v1/sys/policies/{name}"), &body)
                .await?;
            println!();
            success(&format!("Policy {BOLD}{name}{RESET} written."));
            println!();
        }
        PolicyCommands::Read { name } => {
            let resp = client.get(&format!("/v1/sys/policies/{name}")).await?;
            println!();
            print_policy_detail(&name, &resp);
        }
        PolicyCommands::List => {
            let resp = client.get("/v1/sys/policies").await?;
            println!();
            print_policy_list(&resp);
        }
        PolicyCommands::Delete { name } => {
            client.delete(&format!("/v1/sys/policies/{name}")).await?;
            println!();
            success(&format!("Policy {BOLD}{name}{RESET} deleted."));
            println!();
        }
    }
    Ok(())
}

// ── Transit commands ─────────────────────────────────────────────────

async fn cmd_transit(client: &Client, action: TransitCommands) -> Result<()> {
    match action {
        TransitCommands::CreateKey { name } => {
            client
                .post_no_body(&format!("/v1/transit/keys/{name}"))
                .await?;
            println!();
            success(&format!("Transit key {BOLD}{name}{RESET} created."));
            println!();
        }
        TransitCommands::RotateKey { name } => {
            let resp = client
                .post_no_body(&format!("/v1/transit/keys/{name}/rotate"))
                .await?;
            let ver = resp
                .get("new_version")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            println!();
            success(&format!(
                "Key {BOLD}{name}{RESET} rotated to {CYAN}v{ver}{RESET}."
            ));
            println!();
        }
        TransitCommands::Encrypt { key, plaintext } => {
            let body = serde_json::json!({ "plaintext": plaintext });
            let resp = client
                .post(&format!("/v1/transit/encrypt/{key}"), &body)
                .await?;
            println!();
            print_encrypt_response(&resp);
        }
        TransitCommands::Decrypt { key, ciphertext } => {
            let body = serde_json::json!({ "ciphertext": ciphertext });
            let resp = client
                .post(&format!("/v1/transit/decrypt/{key}"), &body)
                .await?;
            println!();
            print_decrypt_response(&resp);
        }
        TransitCommands::ListKeys => {
            let resp = client.get("/v1/transit/keys").await?;
            println!();
            print_transit_key_list(&resp);
        }
        TransitCommands::KeyInfo { name } => {
            let resp = client.get(&format!("/v1/transit/keys/{name}")).await?;
            println!();
            print_transit_key_info(&resp);
        }
    }
    Ok(())
}

// ── Database commands ────────────────────────────────────────────────

async fn cmd_database(client: &Client, action: DatabaseCommands) -> Result<()> {
    match action {
        DatabaseCommands::Configure {
            name,
            plugin,
            connection_url,
        } => {
            let body = serde_json::json!({
                "plugin": plugin,
                "connection_url": connection_url,
            });
            client
                .post(&format!("/v1/database/config/{name}"), &body)
                .await?;
            println!();
            success(&format!("Database connection {BOLD}{name}{RESET} configured."));
            println!();
        }
        DatabaseCommands::CreateRole {
            name,
            db_name,
            creation_statement,
        } => {
            let body = serde_json::json!({
                "db_name": db_name,
                "creation_statements": [creation_statement],
            });
            client
                .post(&format!("/v1/database/roles/{name}"), &body)
                .await?;
            println!();
            success(&format!("Database role {BOLD}{name}{RESET} created."));
            println!();
        }
        DatabaseCommands::Creds { name } => {
            let resp = client.get(&format!("/v1/database/creds/{name}")).await?;
            println!();
            header("🗄️", &format!("Database Credentials: {name}"));
            if let Some(u) = resp.get("username").and_then(Value::as_str) {
                kv_line("Username", u);
            }
            if let Some(p) = resp.get("password").and_then(Value::as_str) {
                kv_line("Password", p);
            }
            if let Some(lease) = resp.get("lease_id").and_then(Value::as_str) {
                kv_line("Lease ID", lease);
            }
            if let Some(dur) = resp.get("lease_duration").and_then(Value::as_i64) {
                kv_line("Lease Duration", &format_duration(dur));
            }
            println!();
        }
        DatabaseCommands::ListRoles => {
            let resp = client.get("/v1/database/roles").await?;
            println!();
            header("🗄️", "Database Roles");
            if let Some(keys) = resp.get("keys").and_then(Value::as_array) {
                if keys.is_empty() {
                    println!("  {DIM}(no roles){RESET}");
                } else {
                    for k in keys {
                        if let Some(name) = k.as_str() {
                            println!("  {CYAN}├─{RESET} {name}");
                        }
                    }
                }
            }
            println!();
        }
        DatabaseCommands::ListConfigs => {
            let resp = client.get("/v1/database/config").await?;
            println!();
            header("🗄️", "Database Connections");
            if let Some(keys) = resp.get("keys").and_then(Value::as_array) {
                if keys.is_empty() {
                    println!("  {DIM}(no connections){RESET}");
                } else {
                    for k in keys {
                        if let Some(name) = k.as_str() {
                            println!("  {CYAN}├─{RESET} {name}");
                        }
                    }
                }
            }
            println!();
        }
    }
    Ok(())
}

// ── PKI commands ─────────────────────────────────────────────────────

async fn cmd_pki(client: &Client, action: PkiCommands) -> Result<()> {
    match action {
        PkiCommands::GenerateRoot {
            common_name,
            ttl_hours,
        } => {
            let body = serde_json::json!({
                "common_name": common_name,
                "ttl_hours": ttl_hours,
            });
            let resp = client.post("/v1/pki/root/generate", &body).await?;
            println!();
            header("🏛️", "Root CA Generated");
            if let Some(cn) = resp.get("common_name").and_then(Value::as_str) {
                kv_line("Common Name", cn);
            }
            if let Some(ttl) = resp.get("ttl_hours").and_then(Value::as_u64) {
                kv_line("Validity", &format!("{ttl} hours"));
            }
            if let Some(cert) = resp.get("certificate").and_then(Value::as_str) {
                let short = if cert.len() > 80 {
                    format!("{}...", &cert[..80])
                } else {
                    cert.to_owned()
                };
                kv_line("Certificate", &short);
            }
            println!();
        }
        PkiCommands::Issue {
            role,
            common_name,
            ttl_hours,
        } => {
            let mut body = serde_json::json!({ "common_name": common_name });
            if let Some(ttl) = ttl_hours {
                body["ttl_hours"] = serde_json::json!(ttl);
            }
            let resp = client
                .post(&format!("/v1/pki/issue/{role}"), &body)
                .await?;
            println!();
            header("📜", "Certificate Issued");
            if let Some(serial) = resp.get("serial_number").and_then(Value::as_str) {
                kv_line("Serial", serial);
            }
            if let Some(exp) = resp.get("expiration").and_then(Value::as_str) {
                kv_line("Expires", exp);
            }
            if let Some(cert) = resp.get("certificate").and_then(Value::as_str) {
                let short = if cert.len() > 80 {
                    format!("{}...", &cert[..80])
                } else {
                    cert.to_owned()
                };
                kv_line("Certificate", &short);
            }
            if resp.get("private_key").and_then(Value::as_str).is_some() {
                kv_line("Private Key", "(included in response)");
            }
            println!();
        }
        PkiCommands::CreateRole {
            name,
            allowed_domains,
            allow_subdomains,
        } => {
            let body = serde_json::json!({
                "allowed_domains": allowed_domains,
                "allow_subdomains": allow_subdomains,
            });
            client
                .post(&format!("/v1/pki/roles/{name}"), &body)
                .await?;
            println!();
            success(&format!("PKI role {BOLD}{name}{RESET} created."));
            println!();
        }
        PkiCommands::ListRoles => {
            let resp = client.get("/v1/pki/roles").await?;
            println!();
            header("🏛️", "PKI Roles");
            if let Some(keys) = resp.get("keys").and_then(Value::as_array) {
                if keys.is_empty() {
                    println!("  {DIM}(no roles){RESET}");
                } else {
                    for k in keys {
                        if let Some(name) = k.as_str() {
                            println!("  {CYAN}├─{RESET} {name}");
                        }
                    }
                }
            }
            println!();
        }
        PkiCommands::ListCerts => {
            let resp = client.get("/v1/pki/certs").await?;
            println!();
            header("📜", "Issued Certificates");
            if let Some(keys) = resp.get("keys").and_then(Value::as_array) {
                if keys.is_empty() {
                    println!("  {DIM}(no certificates){RESET}");
                } else {
                    for k in keys {
                        if let Some(serial) = k.as_str() {
                            println!("  {CYAN}├─{RESET} {serial}");
                        }
                    }
                }
            }
            println!();
        }
    }
    Ok(())
}

// ── AppRole commands ─────────────────────────────────────────────────

async fn cmd_approle(client: &Client, action: AppRoleCommands) -> Result<()> {
    match action {
        AppRoleCommands::CreateRole { name, policies } => {
            let body = serde_json::json!({ "policies": policies });
            let resp = client
                .post(&format!("/v1/auth/approle/role/{name}"), &body)
                .await?;
            println!();
            header("🤖", &format!("AppRole: {name}"));
            if let Some(role_id) = resp.get("role_id").and_then(Value::as_str) {
                kv_line("Role ID", role_id);
            }
            success("Role created.");
            println!();
        }
        AppRoleCommands::RoleId { name } => {
            let resp = client
                .get(&format!("/v1/auth/approle/role/{name}/role-id"))
                .await?;
            println!();
            header("🤖", &format!("AppRole: {name}"));
            if let Some(role_id) = resp.get("role_id").and_then(Value::as_str) {
                kv_line("Role ID", role_id);
            }
            println!();
        }
        AppRoleCommands::SecretId { name } => {
            let resp = client
                .post(
                    &format!("/v1/auth/approle/role/{name}/secret-id"),
                    &serde_json::json!({}),
                )
                .await?;
            println!();
            header("🤖", &format!("AppRole Secret ID: {name}"));
            if let Some(secret_id) = resp.get("secret_id").and_then(Value::as_str) {
                println!();
                println!("  {DIM}Secret ID:{RESET}  {GREEN}{BOLD}{secret_id}{RESET}");
                println!();
                println!("  {YELLOW}⚠  Store this securely. It will NOT be shown again.{RESET}");
            }
            println!();
        }
        AppRoleCommands::Login {
            role_id,
            secret_id,
        } => {
            let body = serde_json::json!({
                "role_id": role_id,
                "secret_id": secret_id,
            });
            let resp = client
                .post_no_auth("/v1/auth/approle/login", &body)
                .await?;
            println!();
            print_token_response(&resp);
        }
        AppRoleCommands::ListRoles => {
            let resp = client.get("/v1/auth/approle/role").await?;
            println!();
            header("🤖", "AppRole Roles");
            if let Some(keys) = resp.get("keys").and_then(Value::as_array) {
                if keys.is_empty() {
                    println!("  {DIM}(no roles){RESET}");
                } else {
                    for k in keys {
                        if let Some(name) = k.as_str() {
                            println!("  {CYAN}├─{RESET} {name}");
                        }
                    }
                }
            }
            println!();
        }
    }
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────

fn parse_kv_pairs(pairs: &[String]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("invalid key=value pair: '{pair}'"))?;
        map.insert(key.to_owned(), value.to_owned());
    }
    Ok(map)
}

fn print_json(value: &Value) {
    if value.is_null() {
        return;
    }
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("failed to format JSON: {e}"),
    }
}
