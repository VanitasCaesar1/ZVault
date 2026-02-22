# zvault

Official Python SDK for [ZVault Cloud](https://zvault.cloud).

## Install

```bash
pip install zvault
```

## Usage

```python
import os
from zvault import ZVault

vault = ZVault(token=os.environ["ZVAULT_TOKEN"])
secrets = vault.get_all(env="production")
db_url = secrets["DATABASE_URL"]
```

### Context manager

```python
with ZVault(token="zvt_...") as vault:
    secrets = vault.get_all("production")
```

### Inject into environment

```python
vault = ZVault(token="zvt_...")
vault.inject_into_env("production")
# All secrets now in os.environ
```

## API

- `ZVault(token, *, base_url, org_id, project_id, default_env, cache_ttl, timeout, max_retries, debug)`
- `vault.get_all(env) -> dict[str, str]`
- `vault.get(key, env) -> str`
- `vault.set(key, value, env, comment) -> SecretEntry`
- `vault.delete(key, env) -> None`
- `vault.list_keys(env) -> list[SecretKey]`
- `vault.inject_into_env(env, overwrite) -> int`
- `vault.healthy() -> HealthStatus`
- `vault.close() -> None`

## License

MIT
