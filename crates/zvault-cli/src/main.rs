//! `ZVault` CLI — command-line client for the `ZVault` secrets manager.
//!
//! A standalone HTTP client that communicates with the `ZVault` server.
//! No internal crate dependencies — talks exclusively via the REST API.

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod license;
mod mcp;
mod setup;

use std::collections::HashMap;
use std::fmt::Write as _;
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
//
// ANSI Shadow style — renders cleanly across all modern terminals.
// Generated with FIGlet "ANSI Shadow" font, hand-trimmed for alignment.

const BANNER: &str = r"
  ███████╗██╗   ██╗ █████╗ ██╗   ██╗██╗  ████████╗
  ╚══███╔╝██║   ██║██╔══██╗██║   ██║██║  ╚══██╔══╝
    ███╔╝ ██║   ██║███████║██║   ██║██║     ██║
   ███╔╝  ╚██╗ ██╔╝██╔══██║██║   ██║██║     ██║
  ███████╗ ╚████╔╝ ██║  ██║╚██████╔╝███████╗██║
  ╚══════╝  ╚═══╝  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝
";

const BANNER_SMALL: &str = "⟐ ZVault";

fn print_banner() {
    println!("{CYAN}{BOLD}{BANNER}{RESET}");
    println!("  {DIM}Secrets management, done right.{RESET}");
    println!("  {DIM}AES-256-GCM · Shamir's Secret Sharing · Zero-Trust{RESET}");
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
    /// Import secrets from a .env file into the vault.
    Import {
        /// Path to the .env file (default: ".env").
        #[arg(default_value = ".env")]
        file: String,
        /// Project name for namespacing secrets (default: current directory name).
        #[arg(long)]
        project: Option<String>,
        /// Skip backing up the original .env file.
        #[arg(long, default_value = "false")]
        no_backup: bool,
        /// Skip generating .env.zvault reference file.
        #[arg(long, default_value = "false")]
        no_ref: bool,
        /// Skip adding .env to .gitignore.
        #[arg(long, default_value = "false")]
        no_gitignore: bool,
    },
    /// Run a command with secrets injected from the vault.
    Run {
        /// Path to .env.zvault (or .env with zvault:// URIs). Default: auto-detect.
        #[arg(long)]
        env_file: Option<String>,
        /// The command and arguments to run.
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
    /// Start the MCP (Model Context Protocol) server for AI assistant integration.
    #[command(name = "mcp-server")]
    McpServer,
    /// Configure an IDE to use ZVault as an MCP server.
    Setup {
        /// IDE to configure: cursor, kiro, continue, or generic.
        ide: String,
    },
    /// Activate a Pro/Team/Enterprise license.
    Activate {
        /// License key (from <https://zvault.cloud/pricing>).
        key: String,
    },
    /// Show current license status.
    License,
    /// Run diagnostics on vault health, license, and MCP connectivity.
    Doctor,
    /// Initialize ZVault for the current project (generate .zvault.toml config).
    #[command(name = "project-init")]
    ProjectInit {
        /// Project name (default: current directory name).
        #[arg(long)]
        name: Option<String>,
        /// Vault server address to use.
        #[arg(long, default_value = "http://127.0.0.1:8200")]
        server: String,
    },
    /// Lease management operations.
    Lease {
        #[command(subcommand)]
        action: LeaseCommands,
    },
    /// Export audit log entries.
    #[command(name = "audit-export")]
    AuditExport {
        /// Output format: json or csv.
        #[arg(long, default_value = "json")]
        format: String,
        /// Maximum entries to export.
        #[arg(long, default_value = "1000")]
        limit: usize,
        /// Output file path (default: stdout).
        #[arg(long)]
        output: Option<String>,
    },
    /// Send a test webhook notification.
    Notify {
        #[command(subcommand)]
        action: NotifyCommands,
    },
    /// Secret rotation operations.
    Rotate {
        #[command(subcommand)]
        action: RotateCommands,
    },
    /// Log in via OIDC (opens browser for Spring authentication).
    Login {
        /// Use OIDC authentication (opens browser).
        #[arg(long)]
        oidc: bool,
    },
    /// Create an encrypted backup of all vault data.
    Backup {
        /// Output file path (default: stdout as JSON).
        #[arg(long)]
        output: Option<String>,
    },
    /// Restore vault data from an encrypted backup.
    Restore {
        /// Path to the backup file.
        file: String,
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

#[derive(Subcommand)]
enum LeaseCommands {
    /// List all active leases.
    List,
    /// Look up a specific lease.
    Lookup {
        /// Lease ID.
        lease_id: String,
    },
    /// Revoke a lease immediately.
    Revoke {
        /// Lease ID.
        lease_id: String,
    },
}

#[derive(Subcommand)]
enum NotifyCommands {
    /// Configure a webhook endpoint for notifications.
    SetWebhook {
        /// Webhook URL (Slack, Discord, or generic).
        url: String,
        /// Events to subscribe to (comma-separated): secret.accessed, secret.rotated, policy.violated, lease.expired.
        #[arg(long, value_delimiter = ',', default_value = "secret.accessed,secret.rotated,lease.expired")]
        events: Vec<String>,
    },
    /// Show current webhook configuration.
    GetWebhook,
    /// Remove webhook configuration.
    RemoveWebhook,
    /// Send a test notification to the configured webhook.
    Test,
}

#[derive(Subcommand)]
enum RotateCommands {
    /// Set a rotation policy for a secret path.
    SetPolicy {
        /// Secret path (e.g., env/myapp/DATABASE_URL).
        path: String,
        /// Rotation interval in hours.
        #[arg(long)]
        interval_hours: u64,
        /// Maximum age before forced rotation (hours).
        #[arg(long)]
        max_age_hours: Option<u64>,
    },
    /// Show rotation policy for a secret path.
    GetPolicy {
        /// Secret path.
        path: String,
    },
    /// List all rotation policies.
    ListPolicies,
    /// Remove a rotation policy.
    RemovePolicy {
        /// Secret path.
        path: String,
    },
    /// Manually trigger rotation for a secret.
    Trigger {
        /// Secret path.
        path: String,
    },
    /// Show rotation status for all secrets with policies.
    Status,
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
        Commands::Import {
            file,
            project,
            no_backup,
            no_ref,
            no_gitignore,
        } => cmd_import(&client, &file, project.as_deref(), no_backup, no_ref, no_gitignore).await,
        Commands::Run { env_file, command } => cmd_run(&client, env_file.as_deref(), &command).await,
        Commands::McpServer => {
            license::require_pro("MCP server (AI Mode)")?;
            mcp::run_mcp_server(client.addr, client.token).await
        }
        Commands::Setup { ide } => {
            license::require_pro("IDE setup (AI Mode)")?;
            cmd_setup(&ide)
        }
        Commands::Activate { key } => cmd_activate(&key).await,
        Commands::License => {
            cmd_license();
            Ok(())
        },
        Commands::Doctor => cmd_doctor(&client).await,
        Commands::ProjectInit { name, server } => cmd_project_init(name.as_deref(), &server),
        Commands::Lease { action } => cmd_lease(&client, action).await,
        Commands::AuditExport { format, limit, output } => {
            cmd_audit_export(&client, &format, limit, output.as_deref()).await
        }
        Commands::Notify { action } => cmd_notify(&client, action).await,
        Commands::Rotate { action } => cmd_rotate(&client, action).await,
        Commands::Login { oidc } => cmd_login(&client, oidc).await,
        Commands::Backup { output } => cmd_backup(&client, output.as_deref()).await,
        Commands::Restore { file } => cmd_restore(&client, &file).await,
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

// ── Import command ────────────────────────────────────────────────────

/// Parse a .env file into key-value pairs.
///
/// Handles:
/// - `KEY=VALUE` (standard)
/// - `KEY="quoted value"` and `KEY='single quoted'`
/// - `# comments` and blank lines (skipped)
/// - `export KEY=VALUE` (strips `export` prefix)
fn parse_env_file(content: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip optional `export ` prefix.
        let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);

        // Split on first `=`.
        let Some((key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };

        let key = key.trim().to_owned();
        if key.is_empty() {
            continue;
        }

        let value = raw_value.trim();

        // Strip surrounding quotes if present.
        let value = if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value[1..value.len().saturating_sub(1).max(1)].to_owned()
        } else {
            value.to_owned()
        };

        entries.push((key, value));
    }

    entries
}

/// Detect the project name from the current directory.
fn detect_project_name() -> Result<String> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("default");
    Ok(name.to_owned())
}

/// Import secrets from a .env file into the vault.
async fn cmd_import(
    client: &Client,
    file: &str,
    project: Option<&str>,
    no_backup: bool,
    no_ref: bool,
    no_gitignore: bool,
) -> Result<()> {
    let path = std::path::Path::new(file);
    if !path.exists() {
        bail!("file not found: {file}");
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {file}"))?;

    let entries = parse_env_file(&content);
    if entries.is_empty() {
        bail!("no secrets found in {file}");
    }

    let project_name = match project {
        Some(p) => p.to_owned(),
        None => detect_project_name()?,
    };

    println!();
    header("📦", &format!("Importing secrets from {file}"));
    println!();
    println!("  {DIM}Project:{RESET}  {BOLD}{project_name}{RESET}");
    println!("  {DIM}Secrets:{RESET}  {BOLD}{}{RESET}", entries.len());
    println!();

    // Store each secret in the vault under env/<project>/<key>.
    let mut imported = 0u32;
    let mut failed = 0u32;

    for (key, value) in &entries {
        let vault_path = format!("env/{project_name}/{key}");
        let body = serde_json::json!({ "data": { "value": value } });

        match client.post(&format!("/v1/secret/data/{vault_path}"), &body).await {
            Ok(_) => {
                println!("  {GREEN}✓{RESET} {key} → {DIM}zvault://env/{project_name}/{key}{RESET}");
                imported = imported.saturating_add(1);
            }
            Err(e) => {
                println!("  {RED}✗{RESET} {key} — {RED}{e}{RESET}");
                failed = failed.saturating_add(1);
            }
        }
    }

    println!();

    // Backup original .env file.
    if !no_backup {
        let backup_path = format!("{file}.backup");
        if let Err(e) = std::fs::copy(file, &backup_path) {
            warning(&format!("failed to backup {file}: {e}"));
        } else {
            success(&format!("Backed up original to {BOLD}{backup_path}{RESET}"));
        }
    }

    // Generate .env.zvault reference file.
    if !no_ref {
        let ref_path = format!(
            "{}{}",
            path.parent()
                .and_then(|p| p.to_str())
                .map(|p| if p.is_empty() { String::new() } else { format!("{p}/") })
                .unwrap_or_default(),
            ".env.zvault"
        );
        let mut ref_content = String::from("# Generated by zvault import — safe to commit\n");
        let _ = writeln!(ref_content, "# Project: {project_name}\n");
        for (key, _) in &entries {
            let _ = writeln!(ref_content, "{key}=zvault://env/{project_name}/{key}");
        }
        if let Err(e) = std::fs::write(&ref_path, &ref_content) {
            warning(&format!("failed to write {ref_path}: {e}"));
        } else {
            success(&format!("Created {BOLD}{ref_path}{RESET} (safe for git)"));
        }
    }

    // Add .env to .gitignore if not already there.
    if !no_gitignore {
        add_to_gitignore(file);
    }

    println!();
    if failed == 0 {
        println!(
            "  {GREEN}{BOLD}✓ Imported {imported} secrets into vault{RESET}"
        );
    } else {
        println!(
            "  {YELLOW}{BOLD}⚠ Imported {imported} secrets, {failed} failed{RESET}"
        );
    }
    println!();

    Ok(())
}

/// Add a file pattern to .gitignore if not already present.
fn add_to_gitignore(pattern: &str) {
    let gitignore = std::path::Path::new(".gitignore");

    if gitignore.exists() {
        if let Ok(content) = std::fs::read_to_string(gitignore) {
            // Check if pattern is already in .gitignore.
            for line in content.lines() {
                if line.trim() == pattern {
                    return;
                }
            }
            // Append to existing .gitignore.
            let addition = if content.ends_with('\n') {
                format!("{pattern}\n")
            } else {
                format!("\n{pattern}\n")
            };
            if let Err(e) = std::fs::write(gitignore, format!("{content}{addition}")) {
                warning(&format!("failed to update .gitignore: {e}"));
                return;
            }
        }
    } else {
        // Create new .gitignore.
        if let Err(e) = std::fs::write(gitignore, format!("{pattern}\n")) {
            warning(&format!("failed to create .gitignore: {e}"));
            return;
        }
    }

    success(&format!("Added {BOLD}{pattern}{RESET} to .gitignore"));
}

// ── Run command ──────────────────────────────────────────────────────

/// Resolve a `zvault://` URI to its secret value from the vault.
async fn resolve_zvault_uri(client: &Client, uri: &str) -> Result<String> {
    let path = uri
        .strip_prefix("zvault://")
        .ok_or_else(|| anyhow::anyhow!("not a zvault:// URI: {uri}"))?;

    let resp = client.get(&format!("/v1/secret/data/{path}")).await?;

    // KV v2 response shape from the HTTP API:
    //   { data: { data: { data: { value: "..." } }, metadata: {...} } }
    //
    // Walk through nested `data` envelopes to reach the actual secret payload.
    let mut node = &resp;
    for _ in 0..4 {
        match node.get("data") {
            Some(inner) => node = inner,
            None => break,
        }
    }

    // Single-value secret stored by `zvault import` (key is "value").
    if let Some(val) = node.get("value").and_then(Value::as_str) {
        return Ok(val.to_owned());
    }
    // If the node itself is a string (edge case), return it directly.
    if let Some(val) = node.as_str() {
        return Ok(val.to_owned());
    }
    // Multi-value secret — serialize as JSON for the env var.
    if node.is_object() {
        return serde_json::to_string(node).context("failed to serialize secret data");
    }

    bail!("no data found at {path}");
}

/// Find the .env.zvault or .env file with zvault:// references.
fn find_env_file(explicit: Option<&str>) -> Result<String> {
    if let Some(path) = explicit {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_owned());
        }
        bail!("env file not found: {path}");
    }

    // Auto-detect: prefer .env.zvault, fall back to .env.
    if std::path::Path::new(".env.zvault").exists() {
        return Ok(".env.zvault".to_owned());
    }
    if std::path::Path::new(".env").exists() {
        return Ok(".env".to_owned());
    }

    bail!("no .env.zvault or .env file found — run `zvault import .env` first");
}

/// Run a command with secrets injected from the vault.
async fn cmd_run(client: &Client, env_file: Option<&str>, command: &[String]) -> Result<()> {
    if command.is_empty() {
        bail!("no command specified — usage: zvault run -- npm run dev");
    }

    let env_path = find_env_file(env_file)?;
    let content = std::fs::read_to_string(&env_path)
        .with_context(|| format!("failed to read {env_path}"))?;

    let entries = parse_env_file(&content);
    if entries.is_empty() {
        bail!("no environment variables found in {env_path}");
    }

    // Resolve zvault:// URIs and collect plain values.
    let mut env_vars: Vec<(String, String)> = Vec::with_capacity(entries.len());
    let mut resolved = 0u32;
    let mut plain = 0u32;

    println!();
    header("🔑", &format!("Resolving secrets from {env_path}"));
    println!();

    for (key, value) in &entries {
        if value.starts_with("zvault://") {
            match resolve_zvault_uri(client, value).await {
                Ok(secret) => {
                    println!("  {GREEN}✓{RESET} {key} {DIM}← {value}{RESET}");
                    env_vars.push((key.clone(), secret));
                    resolved = resolved.saturating_add(1);
                }
                Err(e) => {
                    println!("  {RED}✗{RESET} {key} — {RED}{e}{RESET}");
                    bail!("failed to resolve {key}: {e}");
                }
            }
        } else {
            // Plain value — pass through as-is.
            env_vars.push((key.clone(), value.clone()));
            plain = plain.saturating_add(1);
        }
    }

    println!();
    println!(
        "  {DIM}Resolved {resolved} secrets, {plain} plain values{RESET}"
    );
    println!();

    // Execute the child process with injected environment.
    let program = &command[0];
    let args = &command[1..];

    println!(
        "  {CYAN}{BOLD}▶{RESET} {BOLD}{}{RESET}",
        command.join(" ")
    );
    println!();

    let status = std::process::Command::new(program)
        .args(args)
        .envs(env_vars)
        .status()
        .with_context(|| format!("failed to execute: {program}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("command exited with code {code}");
    }

    Ok(())
}

// ── License commands ──────────────────────────────────────────────────

async fn cmd_activate(key: &str) -> Result<()> {
    println!();
    header("🔑", "Activating License");
    println!();

    // Detect key type: Polar keys have no `.` separator, Ed25519 keys do.
    if license::is_polar_key(key) {
        let lic = license::validate_polar_key(key).await?;

        println!("  {DIM}License ID:{RESET}   {BOLD}{}{RESET}", lic.payload.license_id);
        println!("  {DIM}Tier:{RESET}         {GREEN}{BOLD}{}{RESET}", lic.payload.tier);
        println!("  {DIM}Expires:{RESET}      {}", lic.payload.expires_at);
        println!("  {DIM}Source:{RESET}        Polar.sh");
        println!();
        success("License activated via Polar. AI Mode features are now unlocked.");
    } else {
        // Ed25519-signed key — verify locally.
        let lic = license::verify_license_key(key)?;

        // Save to ~/.zvault/license.key.
        let path = license::save_license(key)?;

        println!("  {DIM}License ID:{RESET}   {BOLD}{}{RESET}", lic.payload.license_id);
        println!("  {DIM}Tier:{RESET}         {GREEN}{BOLD}{}{RESET}", lic.payload.tier);
        println!("  {DIM}Email:{RESET}        {}", lic.payload.email);
        println!("  {DIM}Expires:{RESET}      {}", lic.payload.expires_at);
        println!("  {DIM}Saved to:{RESET}     {}", path.display());
        println!();
        success("License activated. AI Mode features are now unlocked.");
    }

    println!();
    Ok(())
}

fn cmd_license() {
    println!();

    match license::load_license() {
        Ok(Some(lic)) => {
            header("🪪", "License Status");
            println!();
            println!("  {DIM}License ID:{RESET}   {BOLD}{}{RESET}", lic.payload.license_id);
            println!("  {DIM}Tier:{RESET}         {GREEN}{BOLD}{}{RESET}", lic.payload.tier);
            println!("  {DIM}Email:{RESET}        {}", lic.payload.email);
            println!("  {DIM}Issued:{RESET}       {}", lic.payload.issued_at);
            println!("  {DIM}Expires:{RESET}      {}", lic.payload.expires_at);
            println!();

            // Show unlocked features.
            let tier = lic.payload.tier;
            println!("  {BOLD}Unlocked Features:{RESET}");
            println!("  {GREEN}✓{RESET} Local vault, CLI, .env import");
            if tier >= license::Tier::Pro {
                println!("  {GREEN}✓{RESET} AI Mode (MCP server)");
                println!("  {GREEN}✓{RESET} zvault:// references");
                println!("  {GREEN}✓{RESET} IDE setup (Cursor, Kiro, Continue)");
                println!("  {GREEN}✓{RESET} llms.txt generation");
            }
            if tier >= license::Tier::Team {
                println!("  {GREEN}✓{RESET} Shared vault");
                println!("  {GREEN}✓{RESET} OIDC SSO");
                println!("  {GREEN}✓{RESET} Audit log export");
                println!("  {GREEN}✓{RESET} Slack/Discord alerts");
            }
            if tier >= license::Tier::Enterprise {
                println!("  {GREEN}✓{RESET} HA clustering");
                println!("  {GREEN}✓{RESET} K8s operator");
                println!("  {GREEN}✓{RESET} Namespaces");
                println!("  {GREEN}✓{RESET} SLA");
            }
        }
        Ok(None) => {
            header("🪪", "License Status");
            println!();
            println!("  {DIM}Tier:{RESET}         {BOLD}Free{RESET}");
            println!();
            println!("  {DIM}Included:{RESET}");
            println!("  {GREEN}✓{RESET} Local vault, CLI, .env import");
            println!("  {GREEN}✓{RESET} KV, Transit, PKI engines");
            println!("  {GREEN}✓{RESET} Web dashboard");
            println!();
            println!("  {DIM}Locked (Pro $8/mo):{RESET}");
            println!("  {RED}✗{RESET} AI Mode (MCP server)");
            println!("  {RED}✗{RESET} zvault:// references");
            println!("  {RED}✗{RESET} IDE setup & llms.txt");
            println!();
            println!("  {CYAN}Upgrade:{RESET} https://zvault.cloud/pricing");
            println!("  {CYAN}Activate:{RESET} zvault activate <license-key>");
        }
        Err(e) => {
            header("🪪", "License Status");
            println!();
            println!("  {RED}{BOLD}✗{RESET} {RED}License error: {e}{RESET}");
            println!();
            println!("  {DIM}Your license file may be corrupted or expired.{RESET}");
            println!("  {DIM}Re-activate with:{RESET} zvault activate <license-key>");
        }
    }

    println!();
}

// ── IDE setup ────────────────────────────────────────────────────────

fn cmd_setup(ide: &str) -> Result<()> {
    println!();
    header("🔧", &format!("Setting up ZVault for {ide}"));
    println!();

    let target = match ide.to_lowercase().as_str() {
        "cursor" => setup::Ide::Cursor,
        "kiro" => setup::Ide::Kiro,
        "continue" => setup::Ide::Continue,
        "generic" => setup::Ide::Generic,
        other => bail!(
            "unknown IDE: '{other}'. Supported: cursor, kiro, continue, generic"
        ),
    };

    setup::run_setup(target)?;

    println!();
    success("IDE setup complete.");
    println!();
    Ok(())
}

// ── Doctor command ────────────────────────────────────────────────────

/// Run diagnostics on vault health, license status, and MCP connectivity.
async fn cmd_doctor(client: &Client) -> Result<()> {
    println!();
    header("🩺", "ZVault Doctor");
    println!();

    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut warn = 0u32;

    // ── 1. Vault server reachability ─────────────────────────────
    print!("  Vault server ({})... ", client.addr);
    match client.get_no_auth("/v1/sys/health").await {
        Ok(resp) => {
            let initialized = resp.get("initialized").and_then(Value::as_bool).unwrap_or(false);
            let sealed = resp.get("sealed").and_then(Value::as_bool).unwrap_or(true);

            if !initialized {
                println!("{YELLOW}not initialized{RESET}");
                warn = warn.saturating_add(1);
            } else if sealed {
                println!("{YELLOW}sealed{RESET}");
                warn = warn.saturating_add(1);
            } else {
                println!("{GREEN}healthy (unsealed){RESET}");
                pass = pass.saturating_add(1);
            }
        }
        Err(_) => {
            println!("{RED}unreachable{RESET}");
            fail = fail.saturating_add(1);
        }
    }

    // ── 2. Authentication token ──────────────────────────────────
    print!("  Auth token... ");
    match &client.token {
        Some(token) if !token.is_empty() => {
            // Try a token lookup to verify it's valid.
            match client.post("/v1/auth/token/lookup-self", &serde_json::json!({})).await {
                Ok(resp) => {
                    let policies = resp
                        .get("policies")
                        .and_then(Value::as_array)
                        .map(|a| a.len())
                        .unwrap_or(0);
                    println!("{GREEN}valid ({policies} policies){RESET}");
                    pass = pass.saturating_add(1);
                }
                Err(_) => {
                    println!("{YELLOW}set but invalid/expired{RESET}");
                    warn = warn.saturating_add(1);
                }
            }
        }
        _ => {
            println!("{YELLOW}not set (VAULT_TOKEN){RESET}");
            warn = warn.saturating_add(1);
        }
    }

    // ── 3. License status ────────────────────────────────────────
    print!("  License... ");
    match license::load_license() {
        Ok(Some(lic)) => {
            println!(
                "{GREEN}{} (expires {}){RESET}",
                lic.payload.tier, lic.payload.expires_at
            );
            pass = pass.saturating_add(1);
        }
        Ok(None) => {
            println!("{DIM}Free tier{RESET}");
            pass = pass.saturating_add(1);
        }
        Err(e) => {
            println!("{RED}error: {e}{RESET}");
            fail = fail.saturating_add(1);
        }
    }

    // ── 4. MCP server availability ───────────────────────────────
    print!("  MCP server (AI Mode)... ");
    let tier = license::current_tier();
    if tier >= license::Tier::Pro {
        println!("{GREEN}available ({}){RESET}", tier);
        pass = pass.saturating_add(1);
    } else {
        println!("{DIM}locked (requires Pro){RESET}");
        warn = warn.saturating_add(1);
    }

    // ── 5. .env.zvault file ──────────────────────────────────────
    print!("  .env.zvault... ");
    if std::path::Path::new(".env.zvault").exists() {
        let content = std::fs::read_to_string(".env.zvault").unwrap_or_default();
        let uri_count = content.lines().filter(|l| l.contains("zvault://")).count();
        println!("{GREEN}found ({uri_count} references){RESET}");
        pass = pass.saturating_add(1);
    } else if std::path::Path::new(".env").exists() {
        println!("{YELLOW}not found (.env exists — run `zvault import .env`){RESET}");
        warn = warn.saturating_add(1);
    } else {
        println!("{DIM}not found (no .env either){RESET}");
        warn = warn.saturating_add(1);
    }

    // ── 6. .gitignore check ──────────────────────────────────────
    print!("  .gitignore (.env excluded)... ");
    if std::path::Path::new(".gitignore").exists() {
        let content = std::fs::read_to_string(".gitignore").unwrap_or_default();
        if content.lines().any(|l| l.trim() == ".env") {
            println!("{GREEN}yes{RESET}");
            pass = pass.saturating_add(1);
        } else {
            println!("{YELLOW}.env not in .gitignore{RESET}");
            warn = warn.saturating_add(1);
        }
    } else {
        println!("{YELLOW}no .gitignore found{RESET}");
        warn = warn.saturating_add(1);
    }

    // ── 7. IDE MCP config ────────────────────────────────────────
    print!("  IDE MCP config... ");
    let mcp_configs = [
        (".cursor/mcp.json", "Cursor"),
        (".kiro/settings/mcp.json", "Kiro"),
        (".continue/config.json", "Continue"),
    ];
    let mut found_ide: Option<&str> = None;
    for (path, name) in &mcp_configs {
        if std::path::Path::new(path).exists() {
            found_ide = Some(name);
            break;
        }
    }
    match found_ide {
        Some(name) => {
            println!("{GREEN}found ({name}){RESET}");
            pass = pass.saturating_add(1);
        }
        None => {
            println!("{DIM}not configured (run `zvault setup <ide>`){RESET}");
            warn = warn.saturating_add(1);
        }
    }

    // ── Summary ──────────────────────────────────────────────────
    println!();
    println!(
        "  {BOLD}{GREEN}✓ {pass} passed{RESET}  \
         {BOLD}{YELLOW}⚠ {warn} warnings{RESET}  \
         {BOLD}{RED}✗ {fail} failed{RESET}"
    );

    if fail > 0 {
        println!();
        println!("  {DIM}Fix the failures above to get ZVault working properly.{RESET}");
    } else if warn > 0 {
        println!();
        println!("  {DIM}Warnings are non-critical but worth addressing.{RESET}");
    } else {
        println!();
        println!("  {GREEN}Everything looks good.{RESET}");
    }

    println!();
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

// ── Phase 1.3: Project Init ──────────────────────────────────────────

fn cmd_project_init(name: Option<&str>, server: &str) -> Result<()> {
    println!();
    header("📁", "Project Init");
    println!();

    let project_name = match name {
        Some(n) => n.to_owned(),
        None => detect_project_name()?,
    };

    let config_path = std::path::Path::new(".zvault.toml");
    if config_path.exists() {
        warning("  .zvault.toml already exists — skipping");
        println!();
        return Ok(());
    }

    let config_content = format!(
        r#"# ZVault project configuration
# Generated by `zvault project-init`

[project]
name = "{project_name}"

[vault]
# Server address (override with VAULT_ADDR env var)
address = "{server}"

[secrets]
# Default mount path for this project's secrets
mount = "secret"
# Prefix for all secrets in this project
prefix = "env/{project_name}"

[import]
# Files to import secrets from
sources = [".env"]
# Skip backing up original files
no_backup = false

[rotation]
# Default rotation check interval (hours)
check_interval = 24
# Enable rotation notifications
notify = false
"#
    );

    std::fs::write(config_path, &config_content)
        .map_err(|e| anyhow::anyhow!("failed to write .zvault.toml: {e}"))?;

    success(&format!("Created .zvault.toml for project \"{project_name}\""));

    // Add .zvault.toml to .gitignore if it contains tokens/server info
    // Actually, .zvault.toml is safe to commit — it has no secrets.
    // But add .env and .env.backup to .gitignore.
    add_to_gitignore(".env");
    add_to_gitignore(".env.backup");

    println!();
    println!("  {DIM}Next steps:{RESET}");
    println!("    1. Start your vault:  {CYAN}zvault status{RESET}");
    println!("    2. Import secrets:    {CYAN}zvault import .env{RESET}");
    println!("    3. Run your app:      {CYAN}zvault run -- npm run dev{RESET}");
    println!();

    Ok(())
}

// ── Phase 1.5: Lease CLI ─────────────────────────────────────────────

async fn cmd_lease(client: &Client, action: LeaseCommands) -> Result<()> {
    match action {
        LeaseCommands::List => {
            println!();
            header("📋", "Leases");
            println!();

            let resp = client.get("/v1/sys/leases").await?;
            let leases = resp.get("leases").and_then(|v| v.as_array());

            match leases {
                Some(arr) if arr.is_empty() => {
                    println!("  {DIM}No active leases.{RESET}");
                }
                Some(arr) => {
                    println!(
                        "  {DIM}{:<36}  {:<24}  {:<8}  {:<8}  {}{RESET}",
                        "LEASE ID", "ENGINE", "TTL", "RENEW", "STATUS"
                    );
                    for lease in arr {
                        let id = lease.get("lease_id").and_then(|v| v.as_str()).unwrap_or("-");
                        let engine = lease.get("engine_path").and_then(|v| v.as_str()).unwrap_or("-");
                        let ttl = lease.get("ttl_secs").and_then(|v| v.as_i64()).unwrap_or(0);
                        let renewable = lease.get("renewable").and_then(|v| v.as_bool()).unwrap_or(false);
                        let expired = lease.get("expired").and_then(|v| v.as_bool()).unwrap_or(false);

                        let status = if expired {
                            format!("{RED}expired{RESET}")
                        } else {
                            format!("{GREEN}active{RESET}")
                        };
                        let renew_str = if renewable { "yes" } else { "no" };

                        println!(
                            "  {:<36}  {:<24}  {:<8}  {:<8}  {}",
                            id, engine, format_duration(ttl), renew_str, status
                        );
                    }
                    let total = resp.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!();
                    println!("  {DIM}Total: {total} lease(s){RESET}");
                }
                None => {
                    println!("  {DIM}No lease data returned.{RESET}");
                }
            }
            println!();
            Ok(())
        }
        LeaseCommands::Lookup { lease_id } => {
            let resp = client
                .post(
                    "/v1/sys/leases/lookup",
                    &serde_json::json!({ "lease_id": lease_id }),
                )
                .await?;
            println!();
            header("🔍", "Lease Lookup");
            println!();
            kv_line("Lease ID", resp.get("lease_id").and_then(|v| v.as_str()).unwrap_or("-"));
            kv_line("Engine", resp.get("engine_path").and_then(|v| v.as_str()).unwrap_or("-"));
            kv_line("Issued At", resp.get("issued_at").and_then(|v| v.as_str()).unwrap_or("-"));
            let ttl = resp.get("ttl_secs").and_then(|v| v.as_i64()).unwrap_or(0);
            kv_line("TTL", &format_duration(ttl));
            let renewable = resp.get("renewable").and_then(|v| v.as_bool()).unwrap_or(false);
            kv_line("Renewable", if renewable { "yes" } else { "no" });
            let expired = resp.get("expired").and_then(|v| v.as_bool()).unwrap_or(false);
            if expired {
                kv_line("Status", &format!("{RED}expired{RESET}"));
            } else {
                kv_line("Status", &format!("{GREEN}active{RESET}"));
            }
            println!();
            Ok(())
        }
        LeaseCommands::Revoke { lease_id } => {
            client
                .post(
                    "/v1/sys/leases/revoke",
                    &serde_json::json!({ "lease_id": lease_id }),
                )
                .await?;
            println!();
            success(&format!("Lease {lease_id} revoked"));
            println!();
            Ok(())
        }
    }
}

// ── Phase 3.3: Audit Export ──────────────────────────────────────────

async fn cmd_audit_export(
    client: &Client,
    format: &str,
    limit: usize,
    output: Option<&str>,
) -> Result<()> {
    println!();
    header("📊", "Audit Log Export");
    println!();

    let resp = client
        .get_no_auth(&format!("/v1/sys/audit-log?limit={limit}"))
        .await?;

    let entries = resp
        .get("entries")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if entries.is_empty() {
        println!("  {DIM}No audit entries found.{RESET}");
        println!();
        return Ok(());
    }

    let content = match format {
        "csv" => {
            let mut csv = String::from("timestamp,operation,path,actor,status\n");
            for entry in &entries {
                let ts = entry.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
                let op = entry.get("operation").and_then(|v| v.as_str()).unwrap_or("");
                let path = entry.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let actor = entry.get("actor").and_then(|v| v.as_str()).unwrap_or("");
                let status = entry
                    .get("response")
                    .and_then(|v| v.get("status_code"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                csv.push_str(&format!("{ts},{op},{path},{actor},{status}\n"));
            }
            csv
        }
        _ => serde_json::to_string_pretty(&entries)
            .unwrap_or_else(|_| "[]".to_owned()),
    };

    match output {
        Some(path) => {
            std::fs::write(path, &content)
                .map_err(|e| anyhow::anyhow!("failed to write {path}: {e}"))?;
            success(&format!(
                "Exported {} entries to {path} ({format})",
                entries.len()
            ));
        }
        None => {
            println!("{content}");
        }
    }

    println!();
    Ok(())
}

// ── Phase 3.4: Notifications (Webhook) ──────────────────────────────

const WEBHOOK_CONFIG_PATH: &str = ".zvault/webhooks.json";

fn ensure_zvault_dir() -> Result<()> {
    let dir = std::path::Path::new(".zvault");
    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("failed to create .zvault dir: {e}"))?;
    }
    Ok(())
}

fn load_webhook_config() -> Result<serde_json::Value> {
    let path = std::path::Path::new(WEBHOOK_CONFIG_PATH);
    if !path.exists() {
        return Ok(serde_json::json!({ "webhooks": [] }));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read webhook config: {e}"))?;
    serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("invalid webhook config: {e}"))
}

fn save_webhook_config(config: &serde_json::Value) -> Result<()> {
    ensure_zvault_dir()?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| anyhow::anyhow!("failed to serialize webhook config: {e}"))?;
    std::fs::write(WEBHOOK_CONFIG_PATH, content)
        .map_err(|e| anyhow::anyhow!("failed to write webhook config: {e}"))?;
    Ok(())
}

async fn send_webhook(url: &str, payload: &serde_json::Value) -> Result<()> {
    let http = reqwest::Client::new();
    let resp = http
        .post(url)
        .json(payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("webhook request failed: {e}"))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "webhook returned status {}",
            resp.status()
        ))
    }
}

async fn cmd_notify(_client: &Client, action: NotifyCommands) -> Result<()> {
    match action {
        NotifyCommands::SetWebhook { url, events } => {
            println!();
            header("🔔", "Configure Webhook");
            println!();

            let config = serde_json::json!({
                "webhooks": [{
                    "url": url,
                    "events": events,
                    "created_at": chrono_now_iso(),
                }]
            });
            save_webhook_config(&config)?;

            success(&format!("Webhook configured: {url}"));
            println!("  {DIM}Events: {}{RESET}", events.join(", "));
            println!();
            Ok(())
        }
        NotifyCommands::GetWebhook => {
            println!();
            header("🔔", "Webhook Configuration");
            println!();

            let config = load_webhook_config()?;
            let webhooks = config
                .get("webhooks")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if webhooks.is_empty() {
                println!("  {DIM}No webhooks configured.{RESET}");
                println!("  {DIM}Run: zvault notify set-webhook <url>{RESET}");
            } else {
                for wh in &webhooks {
                    let url = wh.get("url").and_then(|v| v.as_str()).unwrap_or("-");
                    let events = wh
                        .get("events")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    kv_line("URL", url);
                    kv_line("Events", &events);
                }
            }
            println!();
            Ok(())
        }
        NotifyCommands::RemoveWebhook => {
            println!();
            let config = serde_json::json!({ "webhooks": [] });
            save_webhook_config(&config)?;
            success("Webhook configuration removed");
            println!();
            Ok(())
        }
        NotifyCommands::Test => {
            println!();
            header("🔔", "Test Webhook");
            println!();

            let config = load_webhook_config()?;
            let webhooks = config
                .get("webhooks")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if webhooks.is_empty() {
                warning("No webhooks configured. Run: zvault notify set-webhook <url>");
                println!();
                return Ok(());
            }

            let payload = serde_json::json!({
                "event": "test",
                "message": "ZVault webhook test notification",
                "timestamp": chrono_now_iso(),
                "vault": "zvault"
            });

            for wh in &webhooks {
                let url = wh.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.is_empty() {
                    continue;
                }
                print!("  Sending to {url}... ");
                match send_webhook(url, &payload).await {
                    Ok(()) => println!("{GREEN}ok{RESET}"),
                    Err(e) => println!("{RED}failed: {e}{RESET}"),
                }
            }
            println!();
            Ok(())
        }
    }
}

// ── Phase 3.2: Secret Rotation ───────────────────────────────────────

const ROTATION_CONFIG_PATH: &str = ".zvault/rotation.json";

fn load_rotation_config() -> Result<serde_json::Value> {
    let path = std::path::Path::new(ROTATION_CONFIG_PATH);
    if !path.exists() {
        return Ok(serde_json::json!({ "policies": {} }));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read rotation config: {e}"))?;
    serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("invalid rotation config: {e}"))
}

fn save_rotation_config(config: &serde_json::Value) -> Result<()> {
    ensure_zvault_dir()?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| anyhow::anyhow!("failed to serialize rotation config: {e}"))?;
    std::fs::write(ROTATION_CONFIG_PATH, content)
        .map_err(|e| anyhow::anyhow!("failed to write rotation config: {e}"))?;
    Ok(())
}

async fn cmd_rotate(client: &Client, action: RotateCommands) -> Result<()> {
    match action {
        RotateCommands::SetPolicy {
            path,
            interval_hours,
            max_age_hours,
        } => {
            println!();
            header("🔄", "Set Rotation Policy");
            println!();

            let mut config = load_rotation_config()?;
            let policies = config
                .get_mut("policies")
                .and_then(|v| v.as_object_mut())
                .ok_or_else(|| anyhow::anyhow!("invalid rotation config"))?;

            policies.insert(
                path.clone(),
                serde_json::json!({
                    "interval_hours": interval_hours,
                    "max_age_hours": max_age_hours,
                    "created_at": chrono_now_iso(),
                    "last_rotated": null,
                }),
            );

            save_rotation_config(&config)?;
            success(&format!(
                "Rotation policy set for {path}: every {interval_hours}h"
            ));
            println!();
            Ok(())
        }
        RotateCommands::GetPolicy { path } => {
            println!();
            header("🔄", "Rotation Policy");
            println!();

            let config = load_rotation_config()?;
            let policy = config
                .get("policies")
                .and_then(|v| v.get(&path));

            match policy {
                Some(p) => {
                    kv_line("Path", &path);
                    let interval = p.get("interval_hours").and_then(|v| v.as_u64()).unwrap_or(0);
                    kv_line("Interval", &format!("{interval}h"));
                    let max_age = p.get("max_age_hours").and_then(|v| v.as_u64());
                    kv_line("Max Age", &max_age.map_or("none".to_owned(), |v| format!("{v}h")));
                    let last = p.get("last_rotated").and_then(|v| v.as_str()).unwrap_or("never");
                    kv_line("Last Rotated", last);
                }
                None => {
                    println!("  {DIM}No rotation policy for {path}{RESET}");
                }
            }
            println!();
            Ok(())
        }
        RotateCommands::ListPolicies => {
            println!();
            header("🔄", "Rotation Policies");
            println!();

            let config = load_rotation_config()?;
            let policies = config
                .get("policies")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            if policies.is_empty() {
                println!("  {DIM}No rotation policies configured.{RESET}");
            } else {
                println!(
                    "  {DIM}{:<40}  {:<12}  {:<12}  {}{RESET}",
                    "PATH", "INTERVAL", "MAX AGE", "LAST ROTATED"
                );
                for (path, policy) in &policies {
                    let interval = policy.get("interval_hours").and_then(|v| v.as_u64()).unwrap_or(0);
                    let max_age = policy
                        .get("max_age_hours")
                        .and_then(|v| v.as_u64())
                        .map_or("-".to_owned(), |v| format!("{v}h"));
                    let last = policy
                        .get("last_rotated")
                        .and_then(|v| v.as_str())
                        .unwrap_or("never");
                    println!(
                        "  {:<40}  {:<12}  {:<12}  {}",
                        path,
                        format!("{interval}h"),
                        max_age,
                        last
                    );
                }
            }
            println!();
            Ok(())
        }
        RotateCommands::RemovePolicy { path } => {
            println!();
            let mut config = load_rotation_config()?;
            if let Some(policies) = config.get_mut("policies").and_then(|v| v.as_object_mut()) {
                policies.remove(&path);
            }
            save_rotation_config(&config)?;
            success(&format!("Rotation policy removed for {path}"));
            println!();
            Ok(())
        }
        RotateCommands::Trigger { path } => {
            println!();
            header("🔄", "Manual Rotation");
            println!();

            // Read the current secret to verify it exists.
            let parts: Vec<&str> = path.splitn(3, '/').collect();
            if parts.len() < 3 {
                return Err(anyhow::anyhow!(
                    "invalid path format — expected env/<project>/<key>"
                ));
            }

            let api_path = format!("/v1/secret/data/{path}");
            let resp = client.get(&api_path).await;
            match resp {
                Ok(_) => {
                    // Update last_rotated timestamp in rotation config.
                    let mut config = load_rotation_config()?;
                    if let Some(policies) =
                        config.get_mut("policies").and_then(|v| v.as_object_mut())
                    {
                        if let Some(policy) = policies.get_mut(&path) {
                            if let Some(obj) = policy.as_object_mut() {
                                obj.insert(
                                    "last_rotated".to_owned(),
                                    serde_json::Value::String(chrono_now_iso()),
                                );
                            }
                        }
                    }
                    save_rotation_config(&config)?;

                    success(&format!("Rotation triggered for {path}"));
                    println!("  {DIM}The secret value should be updated by your rotation handler.{RESET}");
                    println!("  {DIM}Use `zvault kv put {path} value=<new_value>` to update.{RESET}");

                    // Send webhook notification if configured.
                    let wh_config = load_webhook_config()?;
                    let webhooks = wh_config
                        .get("webhooks")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    for wh in &webhooks {
                        let events = wh
                            .get("events")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let has_rotation = events
                            .iter()
                            .any(|e| e.as_str() == Some("secret.rotated"));
                        if has_rotation {
                            if let Some(url) = wh.get("url").and_then(|v| v.as_str()) {
                                let payload = serde_json::json!({
                                    "event": "secret.rotated",
                                    "path": path,
                                    "timestamp": chrono_now_iso(),
                                    "vault": "zvault"
                                });
                                let _ = send_webhook(url, &payload).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("secret not found at {path}: {e}"));
                }
            }
            println!();
            Ok(())
        }
        RotateCommands::Status => {
            println!();
            header("🔄", "Rotation Status");
            println!();

            let config = load_rotation_config()?;
            let policies = config
                .get("policies")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            if policies.is_empty() {
                println!("  {DIM}No rotation policies configured.{RESET}");
                println!();
                return Ok(());
            }

            println!(
                "  {DIM}{:<40}  {:<12}  {}{RESET}",
                "PATH", "INTERVAL", "STATUS"
            );
            for (path, policy) in &policies {
                let interval = policy.get("interval_hours").and_then(|v| v.as_u64()).unwrap_or(0);
                let last = policy.get("last_rotated").and_then(|v| v.as_str());

                let status = match last {
                    None | Some("never") => format!("{YELLOW}never rotated{RESET}"),
                    Some(_ts) => {
                        // Simple status — in production you'd parse the timestamp
                        // and compare against interval_hours.
                        format!("{GREEN}ok{RESET}")
                    }
                };

                println!(
                    "  {:<40}  {:<12}  {}",
                    path,
                    format!("{interval}h"),
                    status
                );
            }
            println!();
            Ok(())
        }
    }
}

// ── Login command ─────────────────────────────────────────────────────

async fn cmd_login(client: &Client, oidc: bool) -> Result<()> {
    if !oidc {
        bail!("only --oidc login is supported — for token auth, set VAULT_TOKEN");
    }

    println!();
    header("🔐", "OIDC Login");
    println!();

    // Check if OIDC is configured on the server.
    let config_resp = client.get_no_auth("/v1/auth/oidc/config").await?;
    let enabled = config_resp
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !enabled {
        bail!("OIDC authentication is not configured on this vault server");
    }

    let login_url = format!("{}/v1/auth/oidc/login", client.addr);
    println!("  {DIM}Opening browser for authentication...{RESET}");
    println!();
    println!("  {CYAN}{login_url}{RESET}");
    println!();

    // Try to open the browser.
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&login_url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&login_url)
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", &login_url])
            .spawn();
    }

    println!("  {DIM}After authenticating, copy the vault token from the dashboard{RESET}");
    println!("  {DIM}and set it with:{RESET}");
    println!();
    println!("    {CYAN}export VAULT_TOKEN=<your-token>{RESET}");
    println!();

    Ok(())
}

// ── Backup command ───────────────────────────────────────────────────

async fn cmd_backup(client: &Client, output: Option<&str>) -> Result<()> {
    println!();
    header("💾", "Vault Backup");
    println!();

    let resp = client.get_no_auth("/v1/sys/backup").await?;

    let entry_count = resp
        .get("entry_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let created_at = resp
        .get("created_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let version = resp
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let content = serde_json::to_string_pretty(&resp)
        .unwrap_or_else(|_| resp.to_string());

    match output {
        Some(path) => {
            std::fs::write(path, &content)
                .with_context(|| format!("failed to write backup to {path}"))?;
            success(&format!("Backup saved to {BOLD}{path}{RESET}"));
        }
        None => {
            println!("{content}");
        }
    }

    println!();
    kv_line("Entries", &entry_count.to_string());
    kv_line("Created", created_at);
    kv_line("Version", version);
    println!();

    if output.is_some() {
        println!("  {YELLOW}⚠  The backup contains encrypted data. Keep it safe.{RESET}");
        println!("  {DIM}Restore with: zvault restore <backup-file>{RESET}");
        println!();
    }

    Ok(())
}

// ── Restore command ──────────────────────────────────────────────────

async fn cmd_restore(client: &Client, file: &str) -> Result<()> {
    println!();
    header("💾", "Vault Restore");
    println!();

    let content = std::fs::read_to_string(file)
        .with_context(|| format!("failed to read backup file: {file}"))?;

    let backup: Value = serde_json::from_str(&content)
        .context("invalid backup file format")?;

    let snapshot = backup
        .get("snapshot")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("backup file missing 'snapshot' field"))?;

    let entry_count = backup
        .get("entry_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    println!("  {DIM}Backup contains {entry_count} entries{RESET}");
    println!("  {YELLOW}⚠  This will overwrite existing vault data.{RESET}");
    println!();

    let body = serde_json::json!({ "snapshot": snapshot });
    let resp = client.post_no_auth("/v1/sys/restore", &body).await?;

    let restored = resp
        .get("entry_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let ok = resp
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if ok {
        success(&format!("Restored {restored} entries from backup"));
        println!();
        println!("  {DIM}Seal and re-unseal the vault to pick up restored state:{RESET}");
        println!("    {CYAN}zvault seal{RESET}");
        println!("    {CYAN}zvault unseal --share <share>{RESET}");
    } else {
        println!("  {RED}Restore failed{RESET}");
    }

    println!();
    Ok(())
}

/// Get current time as ISO 8601 string (no chrono dependency — use simple approach).
fn chrono_now_iso() -> String {
    // We don't have chrono in CLI deps, so use a simple approach.
    // Format: 2026-02-11T12:00:00Z
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Simple UTC timestamp formatting.
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate year/month/day from days since epoch (1970-01-01).
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z"
    )
}

/// Convert days since Unix epoch to (year, month, day).
///
/// Uses Howard Hinnant's civil calendar algorithm. All arithmetic is
/// mathematically proven to stay within `u64` bounds for any valid Unix
/// timestamp (up to year ~5.8 million), so `saturating_*` is used purely
/// as a defensive measure per project coding standards.
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days.saturating_add(719_468);
    let era = z / 146_097;
    let doe = z.saturating_sub(era.saturating_mul(146_097));
    let yoe = (doe.saturating_sub(doe / 1460)
        .saturating_add(doe / 36524)
        .saturating_sub(doe / 146_096))
        / 365;
    let y = yoe.saturating_add(era.saturating_mul(400));
    let doy = doe.saturating_sub(
        365u64
            .saturating_mul(yoe)
            .saturating_add(yoe / 4)
            .saturating_sub(yoe / 100),
    );
    let mp = (5u64.saturating_mul(doy).saturating_add(2)) / 153;
    let d = doy
        .saturating_sub((153u64.saturating_mul(mp).saturating_add(2)) / 5)
        .saturating_add(1);
    let m = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    };
    let y = if m <= 2 { y.saturating_add(1) } else { y };
    (y, m, d)
}
