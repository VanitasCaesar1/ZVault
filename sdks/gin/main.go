// Package zvaultgin provides ZVault middleware for the Gin web framework.
//
// Usage:
//
//	import "github.com/nicosalm/zvault/sdks/gin"
//
//	r := gin.Default()
//	r.Use(zvaultgin.Middleware("production"))
//
//	r.GET("/", func(c *gin.Context) {
//	    secrets := zvaultgin.GetSecrets(c)
//	    dbURL := secrets["DATABASE_URL"]
//	    c.JSON(200, gin.H{"ok": true})
//	})
package zvaultgin

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"sync"
	"time"

	"github.com/gin-gonic/gin"
)

const (
	defaultBaseURL  = "https://api.zvault.cloud"
	defaultTimeout  = 10 * time.Second
	defaultCacheTTL = 5 * time.Minute
	maxRetries      = 2
	secretsKey      = "zvault_secrets"
)

type cachedSecrets struct {
	data      map[string]string
	expiresAt time.Time
}

var (
	cache   *cachedSecrets
	cacheMu sync.RWMutex
)

// Middleware returns a Gin middleware that fetches secrets from ZVault Cloud
// and stores them in the Gin context under "zvault_secrets".
func Middleware(env string) gin.HandlerFunc {
	token := envOr("ZVAULT_TOKEN", "")
	orgID := envOr("ZVAULT_ORG_ID", "")
	projectID := envOr("ZVAULT_PROJECT_ID", "")
	baseURL := envOr("ZVAULT_URL", defaultBaseURL)

	if token == "" || orgID == "" || projectID == "" {
		return func(c *gin.Context) {
			c.Set(secretsKey, map[string]string{})
			c.Next()
		}
	}

	return func(c *gin.Context) {
		cacheMu.RLock()
		if cache != nil && cache.expiresAt.After(time.Now()) {
			c.Set(secretsKey, cache.data)
			cacheMu.RUnlock()
			c.Next()
			return
		}
		cacheMu.RUnlock()

		secrets, err := fetchSecrets(baseURL, token, orgID, projectID, env)
		if err != nil {
			cacheMu.RLock()
			if cache != nil {
				c.Set(secretsKey, cache.data)
			} else {
				c.Set(secretsKey, map[string]string{})
			}
			cacheMu.RUnlock()
			c.Next()
			return
		}

		cacheMu.Lock()
		cache = &cachedSecrets{data: secrets, expiresAt: time.Now().Add(defaultCacheTTL)}
		cacheMu.Unlock()

		c.Set(secretsKey, secrets)
		c.Next()
	}
}

// GetSecrets retrieves the secrets map from the Gin context.
func GetSecrets(c *gin.Context) map[string]string {
	v, ok := c.Get(secretsKey)
	if !ok {
		return map[string]string{}
	}
	s, ok := v.(map[string]string)
	if !ok {
		return map[string]string{}
	}
	return s
}

func fetchSecrets(baseURL, token, orgID, projectID, env string) (map[string]string, error) {
	url := fmt.Sprintf("%s/v1/cloud/orgs/%s/projects/%s/envs/%s/secrets", baseURL, orgID, projectID, env)

	var lastErr error
	client := &http.Client{Timeout: defaultTimeout}

	for i := 0; i <= maxRetries; i++ {
		req, err := http.NewRequest(http.MethodGet, url, nil)
		if err != nil {
			return nil, err
		}
		req.Header.Set("Authorization", "Bearer "+token)
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("User-Agent", "zvault-gin/0.1.0")

		resp, err := client.Do(req)
		if err != nil {
			lastErr = err
			if i < maxRetries {
				time.Sleep(time.Duration(300*(1<<i)) * time.Millisecond)
			}
			continue
		}

		body, _ := io.ReadAll(resp.Body)
		resp.Body.Close()

		if resp.StatusCode >= 200 && resp.StatusCode < 300 {
			return parseSecrets(body), nil
		}

		lastErr = fmt.Errorf("HTTP %d", resp.StatusCode)
		if resp.StatusCode < 500 && resp.StatusCode != 429 {
			return nil, lastErr
		}

		if i < maxRetries {
			time.Sleep(time.Duration(300*(1<<i)) * time.Millisecond)
		}
	}

	return nil, fmt.Errorf("request failed: %w", lastErr)
}

func parseSecrets(body []byte) map[string]string {
	var resp struct {
		Secrets []struct {
			Key   string `json:"key"`
			Value string `json:"value"`
		} `json:"secrets"`
	}
	if err := json.Unmarshal(body, &resp); err != nil {
		return map[string]string{}
	}
	result := make(map[string]string, len(resp.Secrets))
	for _, s := range resp.Secrets {
		result[s.Key] = s.Value
	}
	return result
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
