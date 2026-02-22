package com.zvault;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.time.Instant;
import java.util.Collections;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.ConcurrentHashMap;

/**
 * ZVault Java SDK — fetch and cache secrets from ZVault Cloud.
 *
 * <pre>{@code
 * ZVault vault = ZVault.builder()
 *     .token(System.getenv("ZVAULT_TOKEN"))
 *     .orgId(System.getenv("ZVAULT_ORG_ID"))
 *     .projectId(System.getenv("ZVAULT_PROJECT_ID"))
 *     .build();
 *
 * Map<String, String> secrets = vault.getAll("production");
 * String dbUrl = vault.get("DATABASE_URL", "production");
 * }</pre>
 */
public final class ZVault {

    private static final String DEFAULT_BASE_URL = "https://api.zvault.cloud";
    private static final Duration DEFAULT_TIMEOUT = Duration.ofSeconds(10);
    private static final Duration DEFAULT_CACHE_TTL = Duration.ofMinutes(5);
    private static final int MAX_RETRIES = 2;

    private final String token;
    private final String orgId;
    private final String projectId;
    private final String baseUrl;
    private final Duration cacheTtl;
    private final HttpClient httpClient;

    private final ConcurrentHashMap<String, CacheEntry> cache = new ConcurrentHashMap<>();

    private ZVault(Builder builder) {
        this.token = Objects.requireNonNull(builder.token, "token is required");
        this.orgId = Objects.requireNonNull(builder.orgId, "orgId is required");
        this.projectId = Objects.requireNonNull(builder.projectId, "projectId is required");
        this.baseUrl = builder.baseUrl != null ? builder.baseUrl.replaceAll("/+$", "") : DEFAULT_BASE_URL;
        this.cacheTtl = builder.cacheTtl != null ? builder.cacheTtl : DEFAULT_CACHE_TTL;
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(DEFAULT_TIMEOUT)
                .build();
    }

    public static Builder builder() {
        return new Builder();
    }

    /**
     * Fetch all secrets for the given environment.
     * Results are cached in-memory for the configured TTL.
     */
    public Map<String, String> getAll(String env) throws ZVaultException {
        CacheEntry entry = cache.get(env);
        if (entry != null && entry.expiresAt.isAfter(Instant.now())) {
            return entry.data;
        }

        try {
            String url = String.format("%s/v1/cloud/orgs/%s/projects/%s/envs/%s/secrets",
                    baseUrl, orgId, projectId, env);
            String body = fetchWithRetry(url);

            // Minimal JSON parsing — no external deps
            Map<String, String> secrets = parseSecretsResponse(body);
            cache.put(env, new CacheEntry(secrets, Instant.now().plus(cacheTtl)));
            return secrets;
        } catch (Exception e) {
            // Graceful degradation: return stale cache if available
            if (entry != null) {
                return entry.data;
            }
            throw new ZVaultException("Failed to fetch secrets: " + e.getMessage(), e);
        }
    }

    /**
     * Fetch a single secret by key.
     */
    public String get(String key, String env) throws ZVaultException {
        Map<String, String> all = getAll(env);
        String value = all.get(key);
        if (value == null) {
            throw new ZVaultException("Secret not found: " + key);
        }
        return value;
    }

    /**
     * Check if the ZVault API is reachable.
     */
    public boolean healthy() {
        try {
            HttpRequest req = HttpRequest.newBuilder()
                    .uri(URI.create(baseUrl + "/health"))
                    .timeout(Duration.ofSeconds(5))
                    .GET()
                    .build();
            HttpResponse<String> res = httpClient.send(req, HttpResponse.BodyHandlers.ofString());
            return res.statusCode() == 200;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * Inject all secrets into System properties.
     */
    public int injectIntoSystemProperties(String env) throws ZVaultException {
        Map<String, String> secrets = getAll(env);
        int count = 0;
        for (Map.Entry<String, String> entry : secrets.entrySet()) {
            if (System.getProperty(entry.getKey()) == null) {
                System.setProperty(entry.getKey(), entry.getValue());
                count++;
            }
        }
        return count;
    }

    private String fetchWithRetry(String url) throws ZVaultException {
        Exception lastErr = null;

        for (int i = 0; i <= MAX_RETRIES; i++) {
            try {
                HttpRequest req = HttpRequest.newBuilder()
                        .uri(URI.create(url))
                        .timeout(DEFAULT_TIMEOUT)
                        .header("Authorization", "Bearer " + token)
                        .header("Content-Type", "application/json")
                        .header("User-Agent", "zvault-java/0.1.0")
                        .GET()
                        .build();

                HttpResponse<String> res = httpClient.send(req, HttpResponse.BodyHandlers.ofString());

                if (res.statusCode() >= 200 && res.statusCode() < 300) {
                    return res.body();
                }

                lastErr = new IOException("HTTP " + res.statusCode());
                if (res.statusCode() < 500 && res.statusCode() != 429) {
                    throw new ZVaultException("HTTP " + res.statusCode(), lastErr);
                }
            } catch (ZVaultException e) {
                throw e;
            } catch (Exception e) {
                lastErr = e;
            }

            if (i < MAX_RETRIES) {
                try {
                    Thread.sleep(300L * (1L << i));
                } catch (InterruptedException ie) {
                    Thread.currentThread().interrupt();
                    throw new ZVaultException("Interrupted during retry", ie);
                }
            }
        }

        throw new ZVaultException("Request failed after retries", lastErr);
    }

    /** Minimal JSON parsing for {"secrets":[{"key":"k","value":"v"},...]} */
    private Map<String, String> parseSecretsResponse(String json) {
        Map<String, String> result = new ConcurrentHashMap<>();
        int idx = 0;
        while ((idx = json.indexOf("\"key\"", idx)) != -1) {
            String key = extractJsonString(json, idx);
            int vidx = json.indexOf("\"value\"", idx);
            if (vidx != -1) {
                String value = extractJsonString(json, vidx);
                if (key != null && value != null) {
                    result.put(key, value);
                }
                idx = vidx + 1;
            } else {
                idx++;
            }
        }
        return Collections.unmodifiableMap(result);
    }

    private String extractJsonString(String json, int afterKey) {
        int colon = json.indexOf(':', afterKey);
        if (colon == -1) return null;
        int quote1 = json.indexOf('"', colon + 1);
        if (quote1 == -1) return null;
        int quote2 = json.indexOf('"', quote1 + 1);
        if (quote2 == -1) return null;
        return json.substring(quote1 + 1, quote2);
    }

    private static final class CacheEntry {
        final Map<String, String> data;
        final Instant expiresAt;

        CacheEntry(Map<String, String> data, Instant expiresAt) {
            this.data = data;
            this.expiresAt = expiresAt;
        }
    }

    public static final class Builder {
        private String token;
        private String orgId;
        private String projectId;
        private String baseUrl;
        private Duration cacheTtl;

        public Builder token(String token) { this.token = token; return this; }
        public Builder orgId(String orgId) { this.orgId = orgId; return this; }
        public Builder projectId(String projectId) { this.projectId = projectId; return this; }
        public Builder baseUrl(String baseUrl) { this.baseUrl = baseUrl; return this; }
        public Builder cacheTtl(Duration cacheTtl) { this.cacheTtl = cacheTtl; return this; }

        public ZVault build() {
            return new ZVault(this);
        }
    }
}
