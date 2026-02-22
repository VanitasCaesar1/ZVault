# zvault-gin

ZVault middleware for [Gin](https://gin-gonic.com).

## Install

```bash
go get github.com/nicosalm/zvault/sdks/gin
```

## Quick Start

```go
package main

import (
    "github.com/gin-gonic/gin"
    zvaultgin "github.com/nicosalm/zvault/sdks/gin"
)

func main() {
    r := gin.Default()
    r.Use(zvaultgin.Middleware("production"))

    r.GET("/", func(c *gin.Context) {
        secrets := zvaultgin.GetSecrets(c)
        c.JSON(200, gin.H{"db": secrets["DATABASE_URL"]})
    })

    r.Run(":8080")
}
```

## License

MIT
