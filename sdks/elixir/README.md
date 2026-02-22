# ZVault Elixir SDK

Official Elixir SDK for [ZVault Cloud](https://zvault.cloud) — fetch secrets at runtime with in-memory caching, auto-refresh, and graceful degradation.

## Installation

Add to `mix.exs`:

```elixir
def deps do
  [{:zvault, "~> 0.1.0"}]
end
```

## Quick Start

```elixir
# Add to your Application supervisor
children = [
  {ZVault, token: System.get_env("ZVAULT_TOKEN"),
            org_id: System.get_env("ZVAULT_ORG_ID"),
            project_id: System.get_env("ZVAULT_PROJECT_ID")}
]

Supervisor.start_link(children, strategy: :one_for_one)

# Fetch all secrets
{:ok, secrets} = ZVault.get_all("production")
db_url = Map.get(secrets, "DATABASE_URL")

# Fetch single secret
{:ok, stripe_key} = ZVault.get("STRIPE_KEY", "production")

# Health check
{:ok, %{ok: true}} = ZVault.healthy()
```

## Configuration

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `token` | `ZVAULT_TOKEN` | — | Service token (required) |
| `base_url` | `ZVAULT_URL` | `https://api.zvault.cloud` | API base URL |
| `org_id` | `ZVAULT_ORG_ID` | — | Organization ID |
| `project_id` | `ZVAULT_PROJECT_ID` | — | Project ID |
| `default_env` | `ZVAULT_ENV` | `development` | Default environment |
| `cache_ttl` | — | `300_000` (5 min) | Cache TTL in ms |
| `timeout` | — | `10_000` (10s) | HTTP timeout in ms |
| `max_retries` | — | `3` | Max retry attempts |
| `debug` | — | `false` | Enable debug logging |

## API

| Function | Description |
|----------|-------------|
| `ZVault.get_all(env)` | Fetch all secrets for environment |
| `ZVault.get(key, env)` | Fetch single secret |
| `ZVault.set(key, value, env)` | Set a secret |
| `ZVault.delete(key, env)` | Delete a secret |
| `ZVault.list_keys(env)` | List secret keys (no values) |
| `ZVault.healthy()` | Health check |
| `ZVault.inject_into_env(env)` | Set all secrets as System env vars |

## Features

- GenServer-based with ETS caching
- Automatic retry with exponential backoff
- Graceful degradation (serves cached values on API failure)
- Parallel secret fetching (batched Tasks)
- Zero secret values logged

## License

MIT
