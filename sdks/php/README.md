# ZVault PHP SDK

Official PHP SDK for ZVault Cloud secrets management. Zero external dependencies — uses cURL.

## Install

```bash
composer require zvault/sdk
```

## Quick Start

```php
use ZVault\Client;

$vault = new Client(getenv('ZVAULT_TOKEN'));

// Fetch all secrets
$secrets = $vault->getAll('production');

// Fetch single secret
$dbUrl = $vault->get('DATABASE_URL', 'production');

// Health check
$vault->healthy(); // true

// Inject into environment
$vault->injectIntoEnv('production');
```

## Features

- Zero external dependencies (PHP 8.1+ with cURL)
- In-memory cache with configurable TTL
- Retry with exponential backoff
- Graceful degradation (serves stale cache on failure)

## License

MIT
