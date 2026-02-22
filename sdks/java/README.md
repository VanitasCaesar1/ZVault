# ZVault Java SDK

Official Java/Kotlin SDK for ZVault Cloud secrets management. Zero external dependencies — uses `java.net.http` (Java 17+).

## Install

### Maven
```xml
<dependency>
    <groupId>com.zvault</groupId>
    <artifactId>zvault-sdk</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Gradle (Kotlin DSL)
```kotlin
implementation("com.zvault:zvault-sdk:0.1.0")
```

## Quick Start

```java
import com.zvault.ZVault;

ZVault vault = ZVault.builder()
    .token(System.getenv("ZVAULT_TOKEN"))
    .orgId(System.getenv("ZVAULT_ORG_ID"))
    .projectId(System.getenv("ZVAULT_PROJECT_ID"))
    .build();

// Fetch all secrets
Map<String, String> secrets = vault.getAll("production");

// Fetch single secret
String dbUrl = vault.get("DATABASE_URL", "production");

// Health check
boolean ok = vault.healthy();

// Inject into System properties
vault.injectIntoSystemProperties("production");
```

## Features

- Zero external dependencies (Java 17+ `HttpClient`)
- In-memory cache with configurable TTL
- Retry with exponential backoff
- Graceful degradation (serves stale cache on failure)
- Thread-safe (`ConcurrentHashMap`)

## License

MIT
