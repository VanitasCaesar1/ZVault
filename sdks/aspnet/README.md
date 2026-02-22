# ZVault ASP.NET Core Integration

IConfiguration provider that loads secrets from ZVault Cloud into ASP.NET Core's configuration system.

## Installation

```bash
dotnet add package ZVault.Extensions.Configuration
```

## Quick Start

```csharp
var builder = WebApplication.CreateBuilder(args);

builder.Configuration.AddZVault(options =>
{
    options.Org = "my-company";
    options.Project = "my-saas";
    options.Environment = "production";
    // Token from ZVAULT_TOKEN env var, or set explicitly:
    // options.Token = "zvt_xxx";
});

var app = builder.Build();

// Access secrets via IConfiguration
var dbUrl = app.Configuration["DATABASE_URL"];
var stripeKey = app.Configuration["STRIPE_KEY"];
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `BaseUrl` | `https://api.zvault.cloud` | ZVault Cloud API URL |
| `Token` | `ZVAULT_TOKEN` env var | Service token |
| `Environment` | `development` | Environment to load |
| `Org` | `ZVAULT_ORG` env var | Organization slug |
| `Project` | `ZVAULT_PROJECT` env var | Project slug |
| `Prefix` | `null` | Key prefix (e.g., `"ZVault:"`) |
| `RefreshInterval` | `5 minutes` | Auto-refresh interval (`null` to disable) |
| `Optional` | `false` | Silently fail on load errors |

## Nested Keys

Use `__` in secret names to create nested configuration:

```
DB__HOST=localhost  →  Configuration["DB:HOST"]
DB__PORT=5432       →  Configuration["DB:PORT"]
```

## Dependency Injection

```csharp
// Bind to a typed options class
builder.Services.Configure<DatabaseOptions>(
    builder.Configuration.GetSection("DB"));

public class DatabaseOptions
{
    public string Host { get; set; } = "";
    public int Port { get; set; } = 5432;
}
```

## Optional Mode

Won't throw if ZVault is unreachable (useful for local dev):

```csharp
builder.Configuration.AddZVaultOptional(options =>
{
    options.Environment = "development";
});
```
