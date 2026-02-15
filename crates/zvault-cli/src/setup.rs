//! IDE setup commands for `ZVault` MCP integration.
//!
//! Generates the correct MCP configuration files for each supported IDE
//! so that `zvault mcp-server` is automatically available as an MCP tool provider.
//!
//! Supported IDEs: Cursor, Kiro, Continue, and a generic fallback.

use std::path::Path;

use anyhow::{Context, Result};

/// Which IDE to configure.
#[derive(Clone, Copy)]
pub enum Ide {
    Cursor,
    Kiro,
    Continue,
    Generic,
}

/// Run the setup for the given IDE.
///
/// # Errors
///
/// Returns `Err` if config files cannot be written or merged.
pub fn run_setup(ide: Ide) -> Result<()> {
    match ide {
        Ide::Cursor => setup_cursor(),
        Ide::Kiro => setup_kiro(),
        Ide::Continue => setup_continue(),
        Ide::Generic => setup_generic(),
    }
}

// ── Cursor ───────────────────────────────────────────────────────────

fn setup_cursor() -> Result<()> {
    let dir = Path::new(".cursor");
    ensure_dir(dir)?;

    let config_path = dir.join("mcp.json");
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "zvault": {
                "command": "zvault",
                "args": ["mcp-server"],
                "env": {
                    "VAULT_ADDR": "http://127.0.0.1:8200",
                    "VAULT_TOKEN": "${VAULT_TOKEN}"
                }
            }
        }
    });

    write_json_config(&config_path, &mcp_config)?;

    // Append to .cursorrules if it exists, or create it.
    let rules_path = dir.join("rules");
    ensure_dir(&rules_path)?;
    let zvault_rule = rules_path.join("zvault.mdc");
    let rule_content = "\
---
description: ZVault secrets management rules
globs: **/*.env*, **/docker-compose*, **/.env.zvault
---

# ZVault Integration

- NEVER read or output actual secret values from the vault.
- Use `zvault://` references in .env files instead of real values.
- Use the `zvault_list_secrets` MCP tool to discover available secrets.
- Use the `zvault_describe_secret` MCP tool to check secret metadata.
- Use the `zvault_check_env` MCP tool to verify environment completeness.
- When the user needs to run their app, suggest `zvault run -- <command>`.
- When storing new secrets, use the `zvault_set_secret` MCP tool.
";
    std::fs::write(&zvault_rule, rule_content)
        .with_context(|| format!("failed to write {}", zvault_rule.display()))?;

    println!("  ✓ Created {}", config_path.display());
    println!("  ✓ Created {}", zvault_rule.display());
    println!();
    println!("  Cursor is now configured to use ZVault as an MCP server.");
    println!("  Make sure VAULT_TOKEN is set in your environment.");

    Ok(())
}

// ── Kiro ─────────────────────────────────────────────────────────────

fn setup_kiro() -> Result<()> {
    let settings_dir = Path::new(".kiro").join("settings");
    ensure_dir(&settings_dir)?;

    let config_path = settings_dir.join("mcp.json");
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "zvault": {
                "command": "zvault",
                "args": ["mcp-server"],
                "env": {
                    "VAULT_ADDR": "http://127.0.0.1:8200",
                    "VAULT_TOKEN": "${VAULT_TOKEN}"
                },
                "disabled": false,
                "autoApprove": [
                    "zvault_list_secrets",
                    "zvault_describe_secret",
                    "zvault_check_env",
                    "zvault_generate_env_template",
                    "zvault_vault_status"
                ]
            }
        }
    });

    write_json_config(&config_path, &mcp_config)?;

    // Create steering file.
    let steering_dir = Path::new(".kiro").join("steering");
    ensure_dir(&steering_dir)?;
    let steering_path = steering_dir.join("zvault.md");
    let steering_content = "\
---
inclusion: auto
---

# ZVault Secrets Management

This project uses ZVault for secrets management. Secrets are referenced via `zvault://` URIs.

## Rules

- NEVER read or output actual secret values from the vault.
- Use `zvault://` references in .env files instead of real values.
- The `.env.zvault` file is safe to commit — it contains only references.
- The `.env` file should be in `.gitignore` — it may contain real values.

## Available MCP Tools

- `zvault_list_secrets` — List secret paths (read-only, safe)
- `zvault_describe_secret` — Get metadata about a secret (safe)
- `zvault_check_env` — Verify all env references resolve (safe)
- `zvault_generate_env_template` — Generate .env.zvault template (safe)
- `zvault_set_secret` — Store a new secret (requires confirmation)
- `zvault_delete_secret` — Delete a secret (requires confirmation)
- `zvault_vault_status` — Check vault health (safe)

## Running the App

Always use `zvault run -- <command>` to inject secrets at runtime:
```bash
zvault run -- npm run dev
zvault run -- cargo run
zvault run -- python manage.py runserver
```
";
    std::fs::write(&steering_path, steering_content)
        .with_context(|| format!("failed to write {}", steering_path.display()))?;

    println!("  ✓ Created {}", config_path.display());
    println!("  ✓ Created {}", steering_path.display());
    println!();
    println!("  Kiro is now configured to use ZVault as an MCP server.");
    println!("  Read-only tools are auto-approved. Write tools require confirmation.");
    println!("  Make sure VAULT_TOKEN is set in your environment.");

    Ok(())
}

// ── Continue ─────────────────────────────────────────────────────────

fn setup_continue() -> Result<()> {
    let dir = Path::new(".continue");
    ensure_dir(dir)?;

    let config_path = dir.join("config.json");

    // Continue uses a different config format.
    let config = serde_json::json!({
        "mcpServers": [{
            "name": "zvault",
            "command": "zvault",
            "args": ["mcp-server"],
            "env": {
                "VAULT_ADDR": "http://127.0.0.1:8200",
                "VAULT_TOKEN": "${VAULT_TOKEN}"
            }
        }]
    });

    write_json_config(&config_path, &config)?;

    println!("  ✓ Created {}", config_path.display());
    println!();
    println!("  Continue is now configured to use ZVault as an MCP server.");
    println!("  Make sure VAULT_TOKEN is set in your environment.");

    Ok(())
}

// ── Generic ──────────────────────────────────────────────────────────

fn setup_generic() -> Result<()> {
    // Generate llms.txt and .env.zvault instructions.
    let llms_path = Path::new("llms.txt");
    let content = "\
# ZVault — AI Secrets Integration

This project uses ZVault for secrets management.
Secrets are referenced via `zvault://` URIs in `.env.zvault` files.

## For AI Assistants

- NEVER read or output actual secret values.
- Use `zvault://` references instead of real values.
- The `.env.zvault` file is safe to read — it contains only references.
- To run the app with secrets injected: `zvault run -- <command>`

## MCP Server

Start the MCP server for tool access:
```
zvault mcp-server
```

Available tools: zvault_list_secrets, zvault_describe_secret, zvault_check_env,
zvault_generate_env_template, zvault_set_secret, zvault_delete_secret, zvault_vault_status.

## Quick Start

```bash
zvault import .env          # Import secrets, generate .env.zvault
zvault run -- npm run dev   # Run with secrets injected
```
";
    std::fs::write(llms_path, content)
        .with_context(|| format!("failed to write {}", llms_path.display()))?;

    println!("  ✓ Created {}", llms_path.display());
    println!();
    println!("  Generic setup complete. Add llms.txt to your repo for AI context.");
    println!("  For IDE-specific setup, use: zvault setup cursor|kiro|continue");

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────

fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("failed to create directory: {}", path.display()))?;
    }
    Ok(())
}

fn write_json_config(path: &Path, value: &serde_json::Value) -> Result<()> {
    if path.exists() {
        // Merge with existing config instead of overwriting.
        let existing = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut existing_json: serde_json::Value = serde_json::from_str(&existing)
            .with_context(|| format!("invalid JSON in {}", path.display()))?;

        // Deep merge: add our mcpServers entries without clobbering others.
        if let (Some(existing_servers), Some(new_servers)) = (
            existing_json.get_mut("mcpServers"),
            value.get("mcpServers"),
        ) {
            if let (Some(existing_obj), Some(new_obj)) =
                (existing_servers.as_object_mut(), new_servers.as_object())
            {
                for (k, v) in new_obj {
                    existing_obj.insert(k.clone(), v.clone());
                }
            }
        } else if let Some(obj) = existing_json.as_object_mut()
            && let Some(servers) = value.get("mcpServers")
        {
            obj.insert("mcpServers".into(), servers.clone());
        }

        let pretty = serde_json::to_string_pretty(&existing_json)
            .context("failed to serialize merged config")?;
        std::fs::write(path, pretty)
            .with_context(|| format!("failed to write {}", path.display()))?;

        println!("  ✓ Merged zvault config into {}", path.display());
    } else {
        let pretty =
            serde_json::to_string_pretty(value).context("failed to serialize config")?;
        std::fs::write(path, pretty)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}
