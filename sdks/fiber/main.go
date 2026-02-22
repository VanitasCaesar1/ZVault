// Package zvaultfiber provides ZVault middleware for the Fiber web framework.
//
// Usage:
//
//	import zvaultfiber "github.com/nicosalm/zvault/sdks/fiber"
//
//	app := fiber.New()
//	app.Use(zvaultfiber.Middleware("production"))
//
//	app.Get("/", func(c *fiber.Ctx) error {
//	    secrets := zvaultfiber.GetSecrets(c)
//	    return c.JSON(fiber.Map{"db": secrets["DATABASE_URL"]})
//	})
package zvaultfiber

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"sync"
	"time"

	"github.com/gofiber/fiber/v2"
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

// Middleware returns a Fiber middleware that fetches secrets from ZVault Cloud
// and stores them in c.Locals("zvault_secrets").
func Middleware(env string) fiber.Handler {
	token := envOr("ZVAULT_TOKEN", "")
	orgID := envOr("ZVAULT_ORG_ID", "")
	projectID := envOr("ZVAULT_PROJECT_ID", "")
	baseURL := envOr("ZVAULT_URL", defaultBaseURL)

	if token == "" || orgID == "" || projectID == "" {
		return func(c *fiber.Ctx) error {
			c.Locals(secretsKey, map[string]string{})
			return c.Next()
		}
	}

	return func(c *fiber.Ctx) error {
		cacheMu.RLock()
		if cache != nil && cache.expiresAt.After(time.Now()) {
			c.Locals(secretsKey, cache.data)
			cacheMu.RUnlock()
			return c.Next()
		}
		cacheMu.RUnlock()

		secrets, err := fetchSecrets(baseURL, token, orgID, projectID, env)
		if err != nil {
			cacheMu.RLock()
			if cache != nil {
				c.Locals(secretsKey, cache.data)
			} else {
				c.Locals(secretsKey, map[string]string{})
			}
			cacheMu.RUnlock()
			return c.Next()
		}

		cacheMu.Lock()
		cache = &cachedSecrets{data: secrets, expiresAt: time.Now().Add(defaultCacheTTL)}
		cacheMu.Unlock()

		c.Locals(secretsKey, secrets)
		return c.Next()
	}
}

// GetSecrets retrieves the secrets map from Fiber context.
func GetSecrets(c *fiber.Ctx) map[string]string {
	v := c.Locals(secretsKey)
	if v == nil {
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
		req.Header.Set("User-Agent", "zvault-fiber/0.1.0")

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
