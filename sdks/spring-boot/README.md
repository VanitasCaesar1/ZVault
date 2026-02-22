# ZVault Spring Boot Starter

Auto-configure ZVault secrets as Spring properties.

## Install

```xml
<dependency>
    <groupId>com.zvault</groupId>
    <artifactId>zvault-spring-boot-starter</artifactId>
    <version>0.1.0</version>
</dependency>
```

## Quick Start

```yaml
# application.yml
zvault:
  token: ${ZVAULT_TOKEN}
  org-id: ${ZVAULT_ORG_ID}
  project-id: ${ZVAULT_PROJECT_ID}
  env: production
```

```java
@RestController
public class MyController {

    @Value("${zvault.DATABASE_URL}")
    private String dbUrl;

    @GetMapping("/")
    public String index() {
        return "connected";
    }
}
```

## Features

- Auto-configuration via `@EnableAutoConfiguration`
- PropertySource integration (`@Value("${zvault.KEY}")`)
- Actuator health indicator
- Configurable via `application.yml` or `application.properties`

## License

MIT
