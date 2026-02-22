# ZVault Swift SDK

Official Swift SDK for [ZVault Cloud](https://zvault.cloud) — fetch secrets at runtime with async/await, in-memory caching, and graceful degradation.

## Installation

### Swift Package Manager

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/ArcadeLabsInc/zvault-swift.git", from: "0.1.0")
]
```

## Quick Start

```swift
import ZVault

let vault = ZVaultClient(
    token: ProcessInfo.processInfo.environment["ZVAULT_TOKEN"]!,
    orgId: "my-org",
    projectId: "my-project"
)

// Fetch all secrets
let secrets = try await vault.getAll(env: "production")
let dbUrl = secrets["DATABASE_URL"]

// Fetch single secret
let stripeKey = try await vault.get(key: "STRIPE_KEY", env: "production")

// Health check
let health = await vault.healthy()
print(health.ok) // true
```

## Configuration

| Parameter | Env Var | Default | Description |
|-----------|---------|---------|-------------|
| `token` | `ZVAULT_TOKEN` | — | Service token (required) |
| `baseUrl` | `ZVAULT_URL` | `https://api.zvault.cloud` | API base URL |
| `orgId` | `ZVAULT_ORG_ID` | — | Organization ID |
| `projectId` | `ZVAULT_PROJECT_ID` | — | Project ID |
| `defaultEnv` | `ZVAULT_ENV` | `development` | Default environment |
| `cacheTtl` | — | `300` (5 min) | Cache TTL in seconds |
| `timeout` | — | `10` (10s) | HTTP timeout in seconds |
| `maxRetries` | — | `3` | Max retry attempts |
| `debug` | — | `false` | Enable debug logging |

## API

| Method | Description |
|--------|-------------|
| `getAll(env:)` | Fetch all secrets for environment |
| `get(key:env:)` | Fetch single secret |
| `set(key:value:env:)` | Set a secret |
| `delete(key:env:)` | Delete a secret |
| `listKeys(env:)` | List secret keys (no values) |
| `healthy()` | Health check |

## Features

- Swift concurrency (async/await + TaskGroup)
- Thread-safe NSLock-based cache
- Sendable conformance
- Automatic retry with exponential backoff
- Graceful degradation on API failure
- Zero dependencies (Foundation only)

## Platforms

- macOS 13+
- iOS 16+
- tvOS 16+
- watchOS 9+

## License

MIT
