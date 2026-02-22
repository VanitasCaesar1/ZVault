# ZVault Ruby SDK

Official Ruby SDK for ZVault Cloud secrets management. Zero external dependencies — uses `net/http` and `json` from stdlib.

## Install

```bash
gem install zvault
```

Or in your Gemfile:

```ruby
gem "zvault"
```

## Quick Start

```ruby
require "zvault"

vault = ZVault::Client.new(token: ENV["ZVAULT_TOKEN"])

# Fetch all secrets
secrets = vault.get_all(env: "production")

# Fetch single secret
db_url = vault.get("DATABASE_URL", env: "production")

# Health check
vault.healthy? # => true

# Inject into ENV
vault.inject_into_env(env: "production")
```

## Features

- Zero external dependencies (Ruby 3.1+ stdlib)
- In-memory cache with configurable TTL
- Retry with exponential backoff
- Graceful degradation (serves stale cache on failure)
- Thread-safe (Mutex)

## License

MIT
