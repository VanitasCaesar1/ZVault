"""ZVault SDK client for Python."""

from __future__ import annotations

import os
import sys
import time
import math
import random
from dataclasses import dataclass, field
from typing import Any
from urllib.parse import quote

import httpx

from zvault.errors import (
    ZVaultAPIError,
    ZVaultAuthError,
    ZVaultConfigError,
    ZVaultNotFoundError,
    ZVaultTimeoutError,
)

_DEFAULT_BASE_URL = "https://api.zvault.cloud"
_DEFAULT_CACHE_TTL = 300.0  # 5 minutes
_DEFAULT_TIMEOUT = 10.0  # seconds
_DEFAULT_MAX_RETRIES = 3
_RETRY_BASE_DELAY = 0.5  # seconds


@dataclass
class SecretEntry:
    """A single secret returned by the API."""

    key: str
    value: str
    version: int
    comment: str
    created_at: str
    updated_at: str


@dataclass
class SecretKey:
    """A secret key (no value) from list operations."""

    key: str
    version: int
    comment: str
    updated_at: str


@dataclass
class HealthStatus:
    """Result of a health check."""

    ok: bool
    latency_ms: float
    cached_secrets: int
    last_refresh: float | None


@dataclass
class _CacheEntry:
    secrets: dict[str, str]
    expires_at: float


class ZVault:
    """ZVault SDK client.

    Fetches secrets from ZVault Cloud at runtime with in-memory caching,
    auto-refresh, and graceful degradation.

    Example::

        from zvault import ZVault

        vault = ZVault(token=os.environ["ZVAULT_TOKEN"])
        secrets = vault.get_all(env="production")
        db_url = secrets["DATABASE_URL"]
    """

    def __init__(
        self,
        token: str | None = None,
        *,
        base_url: str | None = None,
        org_id: str | None = None,
        project_id: str | None = None,
        default_env: str | None = None,
        cache_ttl: float = _DEFAULT_CACHE_TTL,
        timeout: float = _DEFAULT_TIMEOUT,
        max_retries: int = _DEFAULT_MAX_RETRIES,
        debug: bool = False,
    ) -> None:
        self._token = token or os.environ.get("ZVAULT_TOKEN", "")
        if not self._token:
            raise ZVaultConfigError(
                "Missing token. Set ZVAULT_TOKEN env var or pass token= argument."
            )

        raw_url = base_url or os.environ.get("ZVAULT_URL", _DEFAULT_BASE_URL)
        self._base_url = raw_url.rstrip("/")
        self._org_id = org_id or os.environ.get("ZVAULT_ORG_ID", "")
        self._project_id = project_id or os.environ.get("ZVAULT_PROJECT_ID", "")
        self._default_env = default_env or os.environ.get("ZVAULT_ENV", "development")
        self._cache_ttl = cache_ttl
        self._max_retries = max_retries
        self._debug = debug
        self._cache: dict[str, _CacheEntry] = {}
        self._last_refresh: float | None = None

        self._client = httpx.Client(
            timeout=timeout,
            headers={
                "Authorization": f"Bearer {self._token}",
                "Content-Type": "application/json",
                "User-Agent": "zvault-python-sdk/0.1.0",
            },
        )

    def get_all(self, env: str | None = None) -> dict[str, str]:
        """Fetch all secrets for an environment.

        Returns a dict of key -> value. Results are cached in-memory.
        On network failure, returns last-known cached values.
        """
        env = self._resolve_env(env)
        self._require_project_config()

        try:
            path = f"/orgs/{self._org_id}/projects/{self._project_id}/envs/{env}/secrets"
            keys_resp = self._request("GET", path)
            keys: list[dict[str, Any]] = keys_resp.get("keys", [])

            secrets: dict[str, str] = {}
            for k in keys:
                try:
                    resp = self._request(
                        "GET", f"{path}/{quote(k['key'], safe='')}"
                    )
                    secret = resp.get("secret", {})
                    secrets[secret["key"]] = secret["value"]
                except ZVaultAPIError:
                    continue

            # Update cache
            self._cache[env] = _CacheEntry(
                secrets=secrets, expires_at=time.monotonic() + self._cache_ttl
            )
            self._last_refresh = time.time()
            self._log(f'Fetched {len(secrets)} secrets for env "{env}"')
            return secrets

        except Exception:
            # Graceful degradation
            cached = self._get_cached(env)
            if cached is not None:
                self._log(
                    f'API unreachable, serving {len(cached)} cached secrets for "{env}"'
                )
                return cached
            raise

    def get(self, key: str, env: str | None = None) -> str:
        """Fetch a single secret by key. Checks cache first.

        Raises:
            ZVaultNotFoundError: If the secret doesn't exist.
        """
        env = self._resolve_env(env)
        self._require_project_config()

        # Check cache
        cached = self._get_cached_key(env, key)
        if cached is not None:
            self._log(f'Cache hit for "{key}" in "{env}"')
            return cached

        try:
            path = (
                f"/orgs/{self._org_id}/projects/{self._project_id}"
                f"/envs/{env}/secrets/{quote(key, safe='')}"
            )
            resp = self._request("GET", path)
            value = resp["secret"]["value"]
            self._set_cached_key(env, key, value)
            return value
        except ZVaultAPIError as e:
            if e.status_code == 404:
                raise ZVaultNotFoundError(key, env) from e
            raise

    def list_keys(self, env: str | None = None) -> list[SecretKey]:
        """List secret keys (no values) for an environment."""
        env = self._resolve_env(env)
        self._require_project_config()

        path = f"/orgs/{self._org_id}/projects/{self._project_id}/envs/{env}/secrets"
        resp = self._request("GET", path)
        return [
            SecretKey(
                key=k["key"],
                version=k["version"],
                comment=k.get("comment", ""),
                updated_at=k.get("updated_at", ""),
            )
            for k in resp.get("keys", [])
        ]

    def set(
        self,
        key: str,
        value: str,
        env: str | None = None,
        comment: str = "",
    ) -> SecretEntry:
        """Set a secret value. Requires write permission."""
        env = self._resolve_env(env)
        self._require_project_config()

        path = (
            f"/orgs/{self._org_id}/projects/{self._project_id}"
            f"/envs/{env}/secrets/{quote(key, safe='')}"
        )
        resp = self._request("PUT", path, json={"value": value, "comment": comment})
        s = resp["secret"]
        self._set_cached_key(env, key, value)
        return SecretEntry(
            key=s["key"],
            value=s["value"],
            version=s["version"],
            comment=s.get("comment", ""),
            created_at=s.get("created_at", ""),
            updated_at=s.get("updated_at", ""),
        )

    def delete(self, key: str, env: str | None = None) -> None:
        """Delete a secret. Requires write permission."""
        env = self._resolve_env(env)
        self._require_project_config()

        path = (
            f"/orgs/{self._org_id}/projects/{self._project_id}"
            f"/envs/{env}/secrets/{quote(key, safe='')}"
        )
        self._request("DELETE", path)

    def inject_into_env(
        self, env: str | None = None, overwrite: bool = False
    ) -> int:
        """Inject all secrets into os.environ.

        Returns the number of variables injected.
        """
        secrets = self.get_all(env)
        count = 0
        for k, v in secrets.items():
            if not overwrite and k in os.environ:
                continue
            os.environ[k] = v
            count += 1
        self._log(f"Injected {count} secrets into os.environ")
        return count

    def healthy(self) -> HealthStatus:
        """Check if the ZVault API is reachable and the token is valid."""
        start = time.monotonic()
        try:
            self._request("GET", "/me")
            ok = True
        except Exception:
            ok = False

        cached_count = sum(
            len(e.secrets)
            for e in self._cache.values()
            if time.monotonic() < e.expires_at
        )

        return HealthStatus(
            ok=ok,
            latency_ms=(time.monotonic() - start) * 1000,
            cached_secrets=cached_count,
            last_refresh=self._last_refresh,
        )

    def close(self) -> None:
        """Close the HTTP client. Call on shutdown."""
        self._client.close()
        self._cache.clear()

    def __enter__(self) -> ZVault:
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    # --- Private ---

    def _resolve_env(self, env: str | None) -> str:
        return env or self._default_env

    def _require_project_config(self) -> None:
        if not self._org_id:
            raise ZVaultConfigError(
                "Missing org_id. Set ZVAULT_ORG_ID env var or pass org_id= argument."
            )
        if not self._project_id:
            raise ZVaultConfigError(
                "Missing project_id. Set ZVAULT_PROJECT_ID env var or pass project_id= argument."
            )

    def _get_cached(self, env: str) -> dict[str, str] | None:
        entry = self._cache.get(env)
        if entry and time.monotonic() < entry.expires_at:
            return dict(entry.secrets)
        return None

    def _get_cached_key(self, env: str, key: str) -> str | None:
        entry = self._cache.get(env)
        if entry and time.monotonic() < entry.expires_at:
            return entry.secrets.get(key)
        return None

    def _set_cached_key(self, env: str, key: str, value: str) -> None:
        entry = self._cache.get(env)
        if entry and time.monotonic() < entry.expires_at:
            entry.secrets[key] = value
        else:
            self._cache[env] = _CacheEntry(
                secrets={key: value},
                expires_at=time.monotonic() + self._cache_ttl,
            )

    def _request(
        self, method: str, path: str, json: Any = None
    ) -> dict[str, Any]:
        url = f"{self._base_url}/v1/cloud{path}"
        last_err: Exception | None = None

        for attempt in range(self._max_retries + 1):
            try:
                resp = self._client.request(method, url, json=json)

                if resp.is_success:
                    if resp.status_code == 204:
                        return {}
                    return resp.json()  # type: ignore[no-any-return]

                # Parse error
                try:
                    body = resp.json()
                    msg = body.get("error", {}).get("message", f"HTTP {resp.status_code}")
                except Exception:
                    msg = f"HTTP {resp.status_code}"

                if resp.status_code in (401, 403):
                    raise ZVaultAuthError(msg)
                if resp.status_code == 404:
                    raise ZVaultAPIError(resp.status_code, msg)

                last_err = ZVaultAPIError(resp.status_code, msg)
                if attempt < self._max_retries and resp.status_code in (
                    429, 500, 502, 503, 504,
                ):
                    self._sleep_with_jitter(attempt)
                    continue

                raise last_err

            except (ZVaultAuthError, ZVaultNotFoundError):
                raise
            except ZVaultAPIError:
                if attempt >= self._max_retries:
                    raise
                continue
            except httpx.TimeoutException as e:
                last_err = ZVaultTimeoutError(str(e))
                if attempt < self._max_retries:
                    self._sleep_with_jitter(attempt)
                    continue
                raise last_err from e
            except httpx.HTTPError as e:
                last_err = ZVaultAPIError(0, str(e))
                if attempt < self._max_retries:
                    self._sleep_with_jitter(attempt)
                    continue
                raise last_err from e

        if last_err:
            raise last_err
        raise ZVaultAPIError(0, "Unknown error")

    def _sleep_with_jitter(self, attempt: int) -> None:
        delay = _RETRY_BASE_DELAY * math.pow(2, attempt)
        jitter = random.random() * delay * 0.3
        time.sleep(delay + jitter)

    def _log(self, message: str) -> None:
        if self._debug:
            print(f"[zvault-sdk] {message}", file=sys.stderr)
