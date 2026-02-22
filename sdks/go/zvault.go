// Package zvault provides the official Go SDK for ZVault Cloud.
//
// Fetch secrets at runtime from ZVault Cloud with in-memory caching,
// auto-refresh, and graceful degradation.
//
// Usage:
//
//	client := zvault.New(os.Getenv("ZVAULT_TOKEN"))
//	secrets, err := client.GetAll(ctx, "production")
//	dbURL := secrets["DATABASE_URL"]
package zvault

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"math"
	"math/rand/v2"
	"net/http"
	"net/url"
	"os"
	"strings"
	"sync"
	"time"
)

const (
	defaultBaseURL    = "https://api.zvault.cloud"
	defaultCacheTTL   = 5 * time.Minute
	defaultTimeout    = 10 * time.Second
	defaultMaxRetries = 3
	retryBaseDelay    = 500 * time.Millisecond
	userAgent         = "zvault-go-sdk/0.1.0"
)

// Config holds configuration for the ZVault client.
type Config struct {
	// Token is the service token (zvt_...) or auth token.
	// Falls back to ZVAULT_TOKEN env var.
	Token string

	// BaseURL is the ZVault Cloud API base URL.
	// Falls back to ZVAULT_URL env var, then https://api.zvault.cloud.
	BaseURL string

	// OrgID is the organization ID.
	// Falls back to ZVAULT_ORG_ID env var.
	OrgID string

	// ProjectID is the project ID.
	// Falls back to ZVAULT_PROJECT_ID env var.
	ProjectID string

	// DefaultEnv is the default environment slug.
	// Falls back to ZVAULT_ENV env var, then "development".
	DefaultEnv string

	// CacheTTL is how long secrets are cached in memory.
	// Default: 5 minutes. Set to 0 to disable caching.
	CacheTTL time.Duration

	// Timeout is the HTTP request timeout. Default: 10 seconds.
	Timeout time.Duration

	// MaxRetries is the number of retry attempts on transient errors.
	// Default: 3.
	MaxRetries int

	// HTTPClient is an optional custom HTTP client. If nil, a default is created.
	HTTPClient *http.Client
}

// SecretEntry represents a single secret from the API.
type SecretEntry struct {
	Key       string `json:"key"`
	Value     string `json:"value"`
	Version   int    `json:"version"`
	Comment   string `json:"comment"`
	CreatedAt string `json:"created_at"`
	UpdatedAt string `json:"updated_at"`
}

// SecretKey represents a secret key (no value) from list operations.
type SecretKey struct {
	Key       string `json:"key"`
	Version   int    `json:"version"`
	Comment   string `json:"comment"`
	UpdatedAt string `json:"updated_at"`
}

// HealthStatus is the result of a health check.
type HealthStatus struct {
	OK            bool
	LatencyMs     int64
	CachedSecrets int
	LastRefresh   time.Time
}

type secretResponse struct {
	Secret SecretEntry `json:"secret"`
}

type secretKeysResponse struct {
	Keys []SecretKey `json:"keys"`
}

type apiErrorBody struct {
	Error *struct {
		Code    int    `json:"code"`
		Message string `json:"message"`
	} `json:"error"`
}

type cacheEntry struct {
	secrets   map[string]string
	expiresAt time.Time
}

// Client is the ZVault SDK client.
type Client struct {
	token      string
	baseURL    string
	orgID      string
	projectID  string
	defaultEnv string
	cacheTTL   time.Duration
	maxRetries int
	httpClient *http.Client

	mu          sync.RWMutex
	cache       map[string]*cacheEntry // env -> cached secrets
	lastRefresh time.Time
}

// New creates a new ZVault client with the given token.
// Additional configuration can be set via NewWithConfig.
func New(token string) *Client {
	return NewWithConfig(Config{Token: token})
}

// NewWithConfig creates a new ZVault client with full configuration.
func NewWithConfig(cfg Config) *Client {
	token := firstNonEmpty(cfg.Token, os.Getenv("ZVAULT_TOKEN"))
	if token == "" {
		panic("zvault: missing token — set ZVAULT_TOKEN env var or pass Config.Token")
	}

	baseURL := firstNonEmpty(cfg.BaseURL, os.Getenv("ZVAULT_URL"), defaultBaseURL)
	baseURL = strings.TrimRight(baseURL, "/")

	orgID := firstNonEmpty(cfg.OrgID, os.Getenv("ZVAULT_ORG_ID"))
	projectID := firstNonEmpty(cfg.ProjectID, os.Getenv("ZVAULT_PROJECT_ID"))
	defaultEnv := firstNonEmpty(cfg.DefaultEnv, os.Getenv("ZVAULT_ENV"), "development")

	cacheTTL := cfg.CacheTTL
	if cacheTTL == 0 {
		cacheTTL = defaultCacheTTL
	}

	timeout := cfg.Timeout
	if timeout == 0 {
		timeout = defaultTimeout
	}

	maxRetries := cfg.MaxRetries
	if maxRetries == 0 {
		maxRetries = defaultMaxRetries
	}

	httpClient := cfg.HTTPClient
	if httpClient == nil {
		httpClient = &http.Client{Timeout: timeout}
	}

	return &Client{
		token:      token,
		baseURL:    baseURL,
		orgID:      orgID,
		projectID:  projectID,
		defaultEnv: defaultEnv,
		cacheTTL:   cacheTTL,
		maxRetries: maxRetries,
		httpClient: httpClient,
		cache:      make(map[string]*cacheEntry),
	}
}

// GetAll fetches all secrets for an environment. Results are cached in-memory.
// On network failure, returns last-known cached values (graceful degradation).
// Pass empty string for env to use the default environment.
func (c *Client) GetAll(ctx context.Context, env string) (map[string]string, error) {
	env = c.resolveEnv(env)
	if err := c.requireProjectConfig(); err != nil {
		return nil, err
	}

	// Fetch key list
	var keysResp secretKeysResponse
	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets", c.orgID, c.projectID, env)
	if err := c.request(ctx, http.MethodGet, path, nil, &keysResp); err != nil {
		// Graceful degradation
		if cached := c.getCached(env); cached != nil {
			return cached, nil
		}
		return nil, err
	}

	// Fetch each secret value
	secrets := make(map[string]string, len(keysResp.Keys))
	for _, k := range keysResp.Keys {
		var resp secretResponse
		secretPath := fmt.Sprintf("%s/%s", path, url.PathEscape(k.Key))
		if err := c.request(ctx, http.MethodGet, secretPath, nil, &resp); err != nil {
			continue // skip individual failures
		}
		secrets[resp.Secret.Key] = resp.Secret.Value
	}

	// Update cache
	c.setCache(env, secrets)
	return secrets, nil
}

// Get fetches a single secret by key. Checks cache first.
// Pass empty string for env to use the default environment.
func (c *Client) Get(ctx context.Context, key, env string) (string, error) {
	env = c.resolveEnv(env)
	if err := c.requireProjectConfig(); err != nil {
		return "", err
	}

	// Check cache
	if val := c.getCachedKey(env, key); val != "" {
		return val, nil
	}

	var resp secretResponse
	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets/%s",
		c.orgID, c.projectID, env, url.PathEscape(key))
	if err := c.request(ctx, http.MethodGet, path, nil, &resp); err != nil {
		return "", err
	}

	// Cache the individual value
	c.setCachedKey(env, key, resp.Secret.Value)
	return resp.Secret.Value, nil
}

// ListKeys lists secret keys (no values) for an environment.
func (c *Client) ListKeys(ctx context.Context, env string) ([]SecretKey, error) {
	env = c.resolveEnv(env)
	if err := c.requireProjectConfig(); err != nil {
		return nil, err
	}

	var resp secretKeysResponse
	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets", c.orgID, c.projectID, env)
	if err := c.request(ctx, http.MethodGet, path, nil, &resp); err != nil {
		return nil, err
	}
	return resp.Keys, nil
}

// Set creates or updates a secret. Requires write permission.
func (c *Client) Set(ctx context.Context, key, value, env, comment string) (*SecretEntry, error) {
	env = c.resolveEnv(env)
	if err := c.requireProjectConfig(); err != nil {
		return nil, err
	}

	body := map[string]string{"value": value, "comment": comment}
	var resp secretResponse
	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets/%s",
		c.orgID, c.projectID, env, url.PathEscape(key))
	if err := c.request(ctx, http.MethodPut, path, body, &resp); err != nil {
		return nil, err
	}

	c.setCachedKey(env, key, value)
	return &resp.Secret, nil
}

// Delete removes a secret. Requires write permission.
func (c *Client) Delete(ctx context.Context, key, env string) error {
	env = c.resolveEnv(env)
	if err := c.requireProjectConfig(); err != nil {
		return err
	}

	path := fmt.Sprintf("/orgs/%s/projects/%s/envs/%s/secrets/%s",
		c.orgID, c.projectID, env, url.PathEscape(key))
	return c.request(ctx, http.MethodDelete, path, nil, nil)
}

// InjectIntoEnv sets all secrets as OS environment variables.
// Existing vars are NOT overwritten unless overwrite is true.
// Returns the number of variables injected.
func (c *Client) InjectIntoEnv(ctx context.Context, env string, overwrite bool) (int, error) {
	secrets, err := c.GetAll(ctx, env)
	if err != nil {
		return 0, err
	}

	count := 0
	for k, v := range secrets {
		if !overwrite && os.Getenv(k) != "" {
			continue
		}
		if err := os.Setenv(k, v); err != nil {
			return count, fmt.Errorf("zvault: failed to set env var %s: %w", k, err)
		}
		count++
	}
	return count, nil
}

// Healthy checks if the ZVault API is reachable and the token is valid.
func (c *Client) Healthy(ctx context.Context) HealthStatus {
	start := time.Now()
	err := c.request(ctx, http.MethodGet, "/me", nil, nil)

	c.mu.RLock()
	cached := 0
	for _, entry := range c.cache {
		if time.Now().Before(entry.expiresAt) {
			cached += len(entry.secrets)
		}
	}
	lastRefresh := c.lastRefresh
	c.mu.RUnlock()

	return HealthStatus{
		OK:            err == nil,
		LatencyMs:     time.Since(start).Milliseconds(),
		CachedSecrets: cached,
		LastRefresh:   lastRefresh,
	}
}

// --- Private methods ---

func (c *Client) resolveEnv(env string) string {
	if env == "" {
		return c.defaultEnv
	}
	return env
}

func (c *Client) requireProjectConfig() error {
	if c.orgID == "" {
		return fmt.Errorf("zvault: missing orgID — set ZVAULT_ORG_ID env var or Config.OrgID")
	}
	if c.projectID == "" {
		return fmt.Errorf("zvault: missing projectID — set ZVAULT_PROJECT_ID env var or Config.ProjectID")
	}
	return nil
}

func (c *Client) getCached(env string) map[string]string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	entry, ok := c.cache[env]
	if !ok || time.Now().After(entry.expiresAt) {
		return nil
	}
	// Return a copy
	result := make(map[string]string, len(entry.secrets))
	for k, v := range entry.secrets {
		result[k] = v
	}
	return result
}

func (c *Client) getCachedKey(env, key string) string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	entry, ok := c.cache[env]
	if !ok || time.Now().After(entry.expiresAt) {
		return ""
	}
	return entry.secrets[key]
}

func (c *Client) setCache(env string, secrets map[string]string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.cache[env] = &cacheEntry{
		secrets:   secrets,
		expiresAt: time.Now().Add(c.cacheTTL),
	}
	c.lastRefresh = time.Now()
}

func (c *Client) setCachedKey(env, key, value string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	entry, ok := c.cache[env]
	if !ok || time.Now().After(entry.expiresAt) {
		entry = &cacheEntry{
			secrets:   make(map[string]string),
			expiresAt: time.Now().Add(c.cacheTTL),
		}
		c.cache[env] = entry
	}
	entry.secrets[key] = value
}

func (c *Client) request(ctx context.Context, method, path string, body any, result any) error {
	fullURL := c.baseURL + "/v1/cloud" + path

	var bodyReader io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("zvault: failed to marshal request body: %w", err)
		}
		bodyReader = strings.NewReader(string(data))
	}

	var lastErr error
	for attempt := 0; attempt <= c.maxRetries; attempt++ {
		req, err := http.NewRequestWithContext(ctx, method, fullURL, bodyReader)
		if err != nil {
			return fmt.Errorf("zvault: failed to create request: %w", err)
		}

		req.Header.Set("Authorization", "Bearer "+c.token)
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("User-Agent", userAgent)

		resp, err := c.httpClient.Do(req)
		if err != nil {
			lastErr = fmt.Errorf("zvault: request failed: %w", err)
			if attempt < c.maxRetries {
				sleepWithJitter(ctx, attempt)
				// Reset body reader for retry
				if body != nil {
					data, _ := json.Marshal(body)
					bodyReader = strings.NewReader(string(data))
				}
				continue
			}
			return lastErr
		}

		defer resp.Body.Close()
		respBody, err := io.ReadAll(resp.Body)
		if err != nil {
			return fmt.Errorf("zvault: failed to read response: %w", err)
		}

		if resp.StatusCode >= 200 && resp.StatusCode < 300 {
			if result != nil && len(respBody) > 0 {
				if err := json.Unmarshal(respBody, result); err != nil {
					return fmt.Errorf("zvault: failed to parse response: %w", err)
				}
			}
			return nil
		}

		// Parse error
		var apiErr apiErrorBody
		_ = json.Unmarshal(respBody, &apiErr)
		msg := fmt.Sprintf("HTTP %d", resp.StatusCode)
		if apiErr.Error != nil && apiErr.Error.Message != "" {
			msg = apiErr.Error.Message
		}

		// Non-retryable errors
		switch resp.StatusCode {
		case http.StatusUnauthorized, http.StatusForbidden:
			return &APIError{StatusCode: resp.StatusCode, Message: msg}
		case http.StatusNotFound:
			return &APIError{StatusCode: resp.StatusCode, Message: msg}
		}

		// Retryable
		lastErr = &APIError{StatusCode: resp.StatusCode, Message: msg}
		if attempt < c.maxRetries && isRetryable(resp.StatusCode) {
			sleepWithJitter(ctx, attempt)
			if body != nil {
				data, _ := json.Marshal(body)
				bodyReader = strings.NewReader(string(data))
			}
			continue
		}

		return lastErr
	}

	return lastErr
}

// APIError represents an error from the ZVault API.
type APIError struct {
	StatusCode int
	Message    string
}

func (e *APIError) Error() string {
	return fmt.Sprintf("zvault: API error %d: %s", e.StatusCode, e.Message)
}

func isRetryable(status int) bool {
	return status == 429 || status == 500 || status == 502 || status == 503 || status == 504
}

func sleepWithJitter(ctx context.Context, attempt int) {
	delay := retryBaseDelay * time.Duration(math.Pow(2, float64(attempt)))
	jitter := time.Duration(rand.Int64N(int64(float64(delay) * 0.3)))
	select {
	case <-time.After(delay + jitter):
	case <-ctx.Done():
	}
}

func firstNonEmpty(vals ...string) string {
	for _, v := range vals {
		if v != "" {
			return v
		}
	}
	return ""
}
