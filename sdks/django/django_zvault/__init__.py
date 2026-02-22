"""
django-zvault — ZVault secrets integration for Django.

Usage in settings.py:
    from django_zvault import load_secrets

    ZVAULT_SECRETS = load_secrets(env="production")

    DATABASES = {
        "default": {
            "ENGINE": "django.db.backends.postgresql",
            "NAME": ZVAULT_SECRETS.get("DB_NAME", "mydb"),
            "HOST": ZVAULT_SECRETS.get("DB_HOST", "localhost"),
        }
    }
"""

from __future__ import annotations

import os
import time
from typing import Any

import httpx

__all__ = ["load_secrets", "ZVaultClient"]

DEFAULT_BASE_URL = "https://api.zvault.cloud"
DEFAULT_TIMEOUT = 10
MAX_RETRIES = 2

_cached: dict[str, str] | None = None
_cached_at: float = 0


class ZVaultError(Exception):
    pass


def load_secrets(
    env: str = "production",
    token: str | None = None,
    org_id: str | None = None,
    project_id: str | None = None,
    base_url: str | None = None,
    inject_env: bool = False,
) -> dict[str, str]:
    """
    Synchronously fetch all secrets for the given environment.
    Designed to be called once at Django startup in settings.py.

    If inject_env=True, also sets secrets as os.environ vars.
    """
    global _cached, _cached_at

    if _cached is not None and (time.time() - _cached_at) < 300:
        return _cached

    _token = token or os.getenv("ZVAULT_TOKEN", "")
    _org_id = org_id or os.getenv("ZVAULT_ORG_ID", "")
    _project_id = project_id or os.getenv("ZVAULT_PROJECT_ID", "")
    _base_url = (base_url or os.getenv("ZVAULT_URL", DEFAULT_BASE_URL)).rstrip("/")

    if not _token or not _org_id or not _project_id:
        return {}

    url = f"{_base_url}/v1/cloud/orgs/{_org_id}/projects/{_project_id}/envs/{env}/secrets"

    last_err: Exception | None = None
    for i in range(MAX_RETRIES + 1):
        try:
            with httpx.Client(timeout=DEFAULT_TIMEOUT) as client:
                res = client.get(
                    url,
                    headers={
                        "Authorization": f"Bearer {_token}",
                        "User-Agent": "django-zvault/0.1.0",
                    },
                )
                if res.is_success:
                    data = res.json()
                    secrets = {s["key"]: s["value"] for s in data.get("secrets", [])}
                    _cached = secrets
                    _cached_at = time.time()

                    if inject_env:
                        for k, v in secrets.items():
                            if k not in os.environ:
                                os.environ[k] = v

                    return secrets

                last_err = ZVaultError(f"HTTP {res.status_code}")
                if res.status_code < 500 and res.status_code != 429:
                    break
        except ZVaultError:
            break
        except Exception as e:
            last_err = e

        if i < MAX_RETRIES:
            time.sleep(0.3 * (2 ** i))

    if _cached is not None:
        return _cached

    return {}


class ZVaultClient:
    """Async client for use in Django views/management commands."""

    def __init__(self, env: str = "production"):
        self._secrets = load_secrets(env=env)

    def get(self, key: str, default: str | None = None) -> str | None:
        return self._secrets.get(key, default)

    def __getitem__(self, key: str) -> str:
        if key not in self._secrets:
            raise KeyError(f"Secret not found: {key}")
        return self._secrets[key]

    def __contains__(self, key: str) -> bool:
        return key in self._secrets

    def all(self) -> dict[str, str]:
        return dict(self._secrets)
