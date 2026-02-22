"""
FastAPI-ZVault — dependency injection for ZVault secrets.

Usage:
    from fastapi import FastAPI, Depends
    from fastapi_zvault import ZVaultDep, get_secret, zvault_lifespan

    app = FastAPI(lifespan=zvault_lifespan)

    @app.get("/")
    async def root(db_url: str = Depends(get_secret("DATABASE_URL"))):
        return {"db": db_url}
"""

from __future__ import annotations

import os
import time
from contextlib import asynccontextmanager
from typing import Any, Callable

import httpx

__all__ = ["ZVaultClient", "ZVaultDep", "get_secret", "zvault_lifespan"]

DEFAULT_BASE_URL = "https://api.zvault.cloud"
DEFAULT_CACHE_TTL = 300  # 5 minutes
DEFAULT_TIMEOUT = 10
MAX_RETRIES = 2

_client: ZVaultClient | None = None


class ZVaultError(Exception):
    pass


class ZVaultClient:
    """ZVault client with in-memory caching and retry."""

    def __init__(
        self,
        token: str | None = None,
        org_id: str | None = None,
        project_id: str | None = None,
        base_url: str | None = None,
        cache_ttl: int = DEFAULT_CACHE_TTL,
    ):
        self.token = token or os.getenv("ZVAULT_TOKEN", "")
        self.org_id = org_id or os.getenv("ZVAULT_ORG_ID", "")
        self.project_id = project_id or os.getenv("ZVAULT_PROJECT_ID", "")
        self.base_url = (base_url or os.getenv("ZVAULT_URL", DEFAULT_BASE_URL)).rstrip("/")
        self.cache_ttl = cache_ttl
        self._cache: dict[str, tuple[dict[str, str], float]] = {}
        self._http = httpx.AsyncClient(
            timeout=DEFAULT_TIMEOUT,
            headers={
                "Authorization": f"Bearer {self.token}",
                "User-Agent": "fastapi-zvault/0.1.0",
            },
        )

    async def get_all(self, env: str = "production") -> dict[str, str]:
        if env in self._cache:
            data, expires = self._cache[env]
            if expires > time.time():
                return data

        try:
            url = f"{self.base_url}/v1/cloud/orgs/{self.org_id}/projects/{self.project_id}/envs/{env}/secrets"
            body = await self._fetch_with_retry(url)
            secrets = {s["key"]: s["value"] for s in body.get("secrets", [])}
            self._cache[env] = (secrets, time.time() + self.cache_ttl)
            return secrets
        except Exception:
            if env in self._cache:
                return self._cache[env][0]
            raise

    async def get(self, key: str, env: str = "production") -> str:
        all_secrets = await self.get_all(env)
        if key not in all_secrets:
            raise ZVaultError(f"Secret not found: {key}")
        return all_secrets[key]

    async def healthy(self) -> bool:
        try:
            res = await self._http.get(f"{self.base_url}/health")
            return res.status_code == 200
        except Exception:
            return False

    async def close(self) -> None:
        await self._http.aclose()

    async def _fetch_with_retry(self, url: str) -> Any:
        last_err: Exception | None = None
        for i in range(MAX_RETRIES + 1):
            try:
                res = await self._http.get(url)
                if res.is_success:
                    return res.json()
                last_err = ZVaultError(f"HTTP {res.status_code}")
                if res.status_code < 500 and res.status_code != 429:
                    raise last_err
            except ZVaultError:
                raise
            except Exception as e:
                last_err = e
            if i < MAX_RETRIES:
                import asyncio
                await asyncio.sleep(0.3 * (2 ** i))
        raise ZVaultError(f"Request failed: {last_err}")


def get_client() -> ZVaultClient:
    global _client
    if _client is None:
        _client = ZVaultClient()
    return _client


@asynccontextmanager
async def zvault_lifespan(app: Any):
    """FastAPI lifespan that initializes and cleans up ZVault client."""
    global _client
    _client = ZVaultClient()
    yield
    await _client.close()
    _client = None


def get_secret(key: str, env: str = "production") -> Callable:
    """FastAPI dependency that resolves a single secret."""
    async def _dep() -> str:
        client = get_client()
        return await client.get(key, env)
    return _dep


class ZVaultDep:
    """FastAPI dependency that provides the full ZVault client."""
    async def __call__(self) -> ZVaultClient:
        return get_client()
