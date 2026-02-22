<?php

declare(strict_types=1);

namespace ZVault;

/**
 * ZVault PHP SDK — fetch and cache secrets from ZVault Cloud.
 *
 * Zero external dependencies — uses cURL.
 *
 * @example
 * $vault = new \ZVault\Client(getenv('ZVAULT_TOKEN'));
 * $secrets = $vault->getAll('production');
 * $dbUrl = $vault->get('DATABASE_URL', 'production');
 */
class Client
{
    private const DEFAULT_BASE_URL = 'https://api.zvault.cloud';
    private const DEFAULT_CACHE_TTL = 300; // 5 minutes
    private const DEFAULT_TIMEOUT = 10;
    private const MAX_RETRIES = 2;

    private string $token;
    private string $orgId;
    private string $projectId;
    private string $baseUrl;
    private int $cacheTtl;

    /** @var array<string, array{data: array<string, string>, expires_at: float}> */
    private array $cache = [];

    public function __construct(
        string $token,
        ?string $orgId = null,
        ?string $projectId = null,
        ?string $baseUrl = null,
        ?int $cacheTtl = null,
    ) {
        $this->token = $token ?: (getenv('ZVAULT_TOKEN') ?: '');
        $this->orgId = $orgId ?: (getenv('ZVAULT_ORG_ID') ?: '');
        $this->projectId = $projectId ?: (getenv('ZVAULT_PROJECT_ID') ?: '');
        $this->baseUrl = rtrim($baseUrl ?: (getenv('ZVAULT_URL') ?: self::DEFAULT_BASE_URL), '/');
        $this->cacheTtl = $cacheTtl ?? self::DEFAULT_CACHE_TTL;

        if (empty($this->token) || empty($this->orgId) || empty($this->projectId)) {
            throw new ZVaultException('token, orgId, and projectId are required');
        }
    }

    /**
     * Fetch all secrets for the given environment.
     *
     * @return array<string, string>
     * @throws ZVaultException
     */
    public function getAll(string $env = 'production'): array
    {
        if (isset($this->cache[$env]) && $this->cache[$env]['expires_at'] > microtime(true)) {
            return $this->cache[$env]['data'];
        }

        try {
            $url = sprintf(
                '%s/v1/cloud/orgs/%s/projects/%s/envs/%s/secrets',
                $this->baseUrl, $this->orgId, $this->projectId, $env
            );
            $body = $this->fetchWithRetry($url);
            $secrets = $this->parseSecrets($body);
            $this->cache[$env] = [
                'data' => $secrets,
                'expires_at' => microtime(true) + $this->cacheTtl,
            ];
            return $secrets;
        } catch (\Throwable $e) {
            if (isset($this->cache[$env])) {
                return $this->cache[$env]['data'];
            }
            throw new ZVaultException('Failed to fetch secrets: ' . $e->getMessage(), 0, $e);
        }
    }

    /**
     * Fetch a single secret by key.
     *
     * @throws ZVaultException
     */
    public function get(string $key, string $env = 'production'): string
    {
        $all = $this->getAll($env);
        if (!array_key_exists($key, $all)) {
            throw new ZVaultException("Secret not found: {$key}");
        }
        return $all[$key];
    }

    /**
     * Check if the ZVault API is reachable.
     */
    public function healthy(): bool
    {
        try {
            $ch = curl_init($this->baseUrl . '/health');
            if ($ch === false) return false;
            curl_setopt_array($ch, [
                CURLOPT_RETURNTRANSFER => true,
                CURLOPT_TIMEOUT => 5,
                CURLOPT_NOBODY => true,
            ]);
            curl_exec($ch);
            $code = curl_getinfo($ch, CURLINFO_HTTP_CODE);
            curl_close($ch);
            return $code === 200;
        } catch (\Throwable) {
            return false;
        }
    }

    /**
     * Inject all secrets into $_ENV and putenv().
     *
     * @return int Number of secrets injected
     */
    public function injectIntoEnv(string $env = 'production'): int
    {
        $secrets = $this->getAll($env);
        $count = 0;
        foreach ($secrets as $k => $v) {
            if (getenv($k) === false) {
                putenv("{$k}={$v}");
                $_ENV[$k] = $v;
                $count++;
            }
        }
        return $count;
    }

    private function fetchWithRetry(string $url): string
    {
        $lastErr = null;

        for ($i = 0; $i <= self::MAX_RETRIES; $i++) {
            try {
                $ch = curl_init($url);
                if ($ch === false) {
                    throw new ZVaultException('Failed to init cURL');
                }

                curl_setopt_array($ch, [
                    CURLOPT_RETURNTRANSFER => true,
                    CURLOPT_TIMEOUT => self::DEFAULT_TIMEOUT,
                    CURLOPT_HTTPHEADER => [
                        'Authorization: Bearer ' . $this->token,
                        'Content-Type: application/json',
                        'User-Agent: zvault-php/0.1.0',
                    ],
                ]);

                $body = curl_exec($ch);
                $code = curl_getinfo($ch, CURLINFO_HTTP_CODE);
                $err = curl_error($ch);
                curl_close($ch);

                if ($body === false) {
                    throw new ZVaultException("cURL error: {$err}");
                }

                if ($code >= 200 && $code < 300) {
                    return (string) $body;
                }

                $lastErr = new ZVaultException("HTTP {$code}");
                if ($code < 500 && $code !== 429) {
                    throw $lastErr;
                }
            } catch (ZVaultException $e) {
                throw $e;
            } catch (\Throwable $e) {
                $lastErr = $e;
            }

            if ($i < self::MAX_RETRIES) {
                usleep((int) (300_000 * (2 ** $i)));
            }
        }

        throw new ZVaultException('Request failed after retries: ' . ($lastErr?->getMessage() ?? 'unknown'));
    }

    /**
     * @return array<string, string>
     */
    private function parseSecrets(string $json): array
    {
        $data = json_decode($json, true);
        $result = [];
        foreach (($data['secrets'] ?? []) as $s) {
            if (isset($s['key'], $s['value'])) {
                $result[$s['key']] = $s['value'];
            }
        }
        return $result;
    }
}

class ZVaultException extends \RuntimeException {}
