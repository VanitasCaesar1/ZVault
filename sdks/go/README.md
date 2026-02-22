# zvault-go

Official Go SDK for [ZVault Cloud](https://zvault.cloud).

## Install

```bash
go get github.com/ArcadeLabsInc/zvault-go
```

## Usage

```go
package main

import (
	"context"
	"fmt"
	"os"

	zvault "github.com/ArcadeLabsInc/zvault-go"
)

func main() {
	client := zvault.New(os.Getenv("ZVAULT_TOKEN"))

	ctx := context.Background()
	secrets, err := client.GetAll(ctx, "production")
	if err != nil {
		panic(err)
	}

	fmt.Println("DB:", secrets["DATABASE_URL"])
}
```

## Configuration

```go
client := zvault.NewWithConfig(zvault.Config{
	Token:      os.Getenv("ZVAULT_TOKEN"),
	OrgID:      "org-123",
	ProjectID:  "proj-456",
	DefaultEnv: "production",
	CacheTTL:   10 * time.Minute,
	Timeout:    5 * time.Second,
	MaxRetries: 3,
})
```

## API

- `New(token) *Client` — create client with token
- `NewWithConfig(cfg) *Client` — create client with full config
- `GetAll(ctx, env) (map[string]string, error)` — fetch all secrets
- `Get(ctx, key, env) (string, error)` — fetch single secret
- `Set(ctx, key, value, env, comment) (*SecretEntry, error)` — set secret
- `Delete(ctx, key, env) error` — delete secret
- `ListKeys(ctx, env) ([]SecretKey, error)` — list keys
- `InjectIntoEnv(ctx, env, overwrite) (int, error)` — inject into os env
- `Healthy(ctx) HealthStatus` — health check

## License

MIT
