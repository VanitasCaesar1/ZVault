# zvault-fiber

ZVault middleware for [Fiber](https://gofiber.io).

## Install

```bash
go get github.com/nicosalm/zvault/sdks/fiber
```

## Quick Start

```go
package main

import (
    "github.com/gofiber/fiber/v2"
    zvaultfiber "github.com/nicosalm/zvault/sdks/fiber"
)

func main() {
    app := fiber.New()
    app.Use(zvaultfiber.Middleware("production"))

    app.Get("/", func(c *fiber.Ctx) error {
        secrets := zvaultfiber.GetSecrets(c)
        return c.JSON(fiber.Map{"db": secrets["DATABASE_URL"]})
    })

    app.Listen(":8080")
}
```

## License

MIT
