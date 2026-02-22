<?php

declare(strict_types=1);

namespace ZVault\Laravel;

use Illuminate\Support\ServiceProvider;

/**
 * ZVault Laravel Service Provider.
 *
 * Loads secrets from ZVault Cloud and merges them into config('zvault.*').
 *
 * Usage:
 *   config('zvault.DATABASE_URL')
 *   config('zvault.STRIPE_KEY')
 */
class ZVaultServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->mergeConfigFrom(__DIR__ . '/../config/zvault.php', 'zvault');

        $this->app->singleton(ZVaultClient::class, function ($app) {
            $config = $app['config']['zvault'];
            return new ZVaultClient(
                token: $config['token'] ?? '',
                orgId: $config['org_id'] ?? '',
                projectId: $config['project_id'] ?? '',
                baseUrl: $config['base_url'] ?? 'https://api.zvault.cloud',
            );
        });
    }

    public function boot(): void
    {
        $this->publishes([
            __DIR__ . '/../config/zvault.php' => config_path('zvault.php'),
        ], 'zvault-config');

        if (config('zvault.enabled', true)) {
            try {
                /** @var ZVaultClient $client */
                $client = $this->app->make(ZVaultClient::class);
                $env = config('zvault.env', 'production');
                $secrets = $client->getAll($env);

                // Merge secrets into config
                foreach ($secrets as $key => $value) {
                    config(["zvault.secrets.{$key}" => $value]);
                }

                if (config('zvault.inject_env', false)) {
                    foreach ($secrets as $key => $value) {
                        if (getenv($key) === false) {
                            putenv("{$key}={$value}");
                        }
                    }
                }
            } catch (\Throwable $e) {
                logger()?->warning("[zvault] Failed to load secrets: {$e->getMessage()}");
            }
        }
    }
}

/**
 * Minimal ZVault client for Laravel.
 */
class ZVaultClient
{
    private const DEFAULT_TIMEOUT = 10;
    private const MAX_RETRIES = 2;

    public function __construct(
        private string $token,
        private string $orgId,
        private string $projectId,
        private string $baseUrl = 'https://api.zvault.cloud',
    ) {}

    /** @return array<string, string> */
    public function getAll(string $env = 'production'): array
    {
        $url = sprintf(
            '%s/v1/cloud/orgs/%s/projects/%s/envs/%s/secrets',
            rtrim($this->baseUrl, '/'), $this->orgId, $this->projectId, $env
        );

        for ($i = 0; $i <= self::MAX_RETRIES; $i++) {
            $ch = curl_init($url);
            if ($ch === false) continue;

            curl_setopt_array($ch, [
                CURLOPT_RETURNTRANSFER => true,
                CURLOPT_TIMEOUT => self::DEFAULT_TIMEOUT,
                CURLOPT_HTTPHEADER => [
                    'Authorization: Bearer ' . $this->token,
                    'Content-Type: application/json',
                    'User-Agent: zvault-laravel/0.1.0',
                ],
            ]);

            $body = curl_exec($ch);
            $code = curl_getinfo($ch, CURLINFO_HTTP_CODE);
            curl_close($ch);

            if ($body !== false && $code >= 200 && $code < 300) {
                $data = json_decode((string) $body, true);
                $result = [];
                foreach (($data['secrets'] ?? []) as $s) {
                    if (isset($s['key'], $s['value'])) {
                        $result[$s['key']] = $s['value'];
                    }
                }
                return $result;
            }

            if ($code < 500 && $code !== 429) break;
            if ($i < self::MAX_RETRIES) usleep((int)(300_000 * (2 ** $i)));
        }

        return [];
    }
}
