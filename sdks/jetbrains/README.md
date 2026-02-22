# ZVault JetBrains Plugin

IntelliJ IDEA, WebStorm, GoLand, PyCharm, and all JetBrains IDEs.

## Features

- **Secret Peek**: Hover over `zvault://` URIs to see secret metadata (not values)
- **Go-to-Definition**: Ctrl+Click on `zvault://project/key` to open in ZVault dashboard
- **Autocomplete**: Secret key suggestions when typing `zvault://` or `process.env.`
- **Gutter Icons**: Visual indicators for lines referencing ZVault secrets
- **Run Configuration**: Inject secrets into run/debug configurations
- **Tool Window**: Browse project secrets without leaving the IDE

## Installation

1. Open Settings → Plugins → Marketplace
2. Search "ZVault"
3. Install and restart

## Configuration

Settings → Tools → ZVault:

| Setting | Default | Description |
|---------|---------|-------------|
| Token | `ZVAULT_TOKEN` env var | Service token or vault token |
| Server URL | `https://api.zvault.cloud` | ZVault API URL |
| Environment | `development` | Default environment |
| Auto-refresh | `5 min` | Secret metadata refresh interval |

## Run Configuration Integration

Add ZVault to any run configuration:

1. Edit Configuration → Environment Variables
2. Click "ZVault" button
3. Select environment
4. Secrets are injected at runtime (never stored in `.idea/`)

## Secret References

The plugin recognizes these patterns:

```
zvault://my-project/DATABASE_URL          # Full URI
process.env.DATABASE_URL                  # Node.js
os.environ["DATABASE_URL"]                # Python
System.getenv("DATABASE_URL")             # Java
os.Getenv("DATABASE_URL")                 # Go
std::env::var("DATABASE_URL")             # Rust
```

## Build

```bash
./gradlew buildPlugin
```

## Compatibility

- IntelliJ IDEA 2024.1+
- WebStorm 2024.1+
- GoLand 2024.1+
- PyCharm 2024.1+
- Rider 2024.1+
- RubyMine 2024.1+
- PhpStorm 2024.1+
