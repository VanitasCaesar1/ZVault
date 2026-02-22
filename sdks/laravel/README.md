# zvault-laravel

ZVault integration for Laravel — `config('zvault.secrets.DATABASE_URL')`.

## Install

```bash
composer require zvault/laravel
php artisan vendor:publish --tag=zvault-config
```

## Quick Start

```php
// .env
ZVAULT_TOKEN=zvt_your_service_token
ZVAULT_ORG_ID=org_xxx
ZVAULT_PROJECT_ID=proj_xxx
ZVAULT_ENV=production

// Anywhere in your app
$dbUrl = config('zvault.secrets.DATABASE_URL');
```

## Features

- Auto-discovery (Laravel 10+)
- Secrets available via `config('zvault.secrets.*')`
- Optional env injection (`ZVAULT_INJECT_ENV=true`)
- Graceful degradation on API failure

## License

MIT
