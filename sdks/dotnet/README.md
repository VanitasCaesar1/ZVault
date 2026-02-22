# ZVault .NET SDK

Official .NET SDK for ZVault Cloud secrets management. Zero external dependencies — uses `System.Net.Http` and `System.Text.Json`.

## Install

```bash
dotnet add package ZVault.SDK
```

## Quick Start

```csharp
using ZVault;

var vault = new ZVaultClient(Environment.GetEnvironmentVariable("ZVAULT_TOKEN")!)
{
    OrgId = Environment.GetEnvironmentVariable("ZVAULT_ORG_ID")!,
    ProjectId = Environment.GetEnvironmentVariable("ZVAULT_PROJECT_ID")!,
};

// Fetch all secrets
var secrets = await vault.GetAllAsync("production");

// Fetch single secret
var dbUrl = await vault.GetAsync("DATABASE_URL", "production");

// Health check
var ok = await vault.HealthyAsync();

// Inject into environment variables
await vault.InjectIntoEnvAsync("production");
```

## Features

- Zero external dependencies (.NET 8+)
- In-memory cache with configurable TTL
- Retry with exponential backoff
- Graceful degradation (serves stale cache on failure)
- Thread-safe (`ConcurrentDictionary`)
- `IDisposable` for proper cleanup

## License

MIT
