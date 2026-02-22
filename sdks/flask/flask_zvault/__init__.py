"""
flask-zvault — ZVault secrets integration for Flask.

Usage:
    from flask import Flask
    from flask_zvault import ZVault

    app = Flask(__name__)
    vault = ZVault(app, env="production")

    @app.route("/")
    def index():
        db_url = app.config["DATABASE_URL"]
        return "ok"
"""

from __future__ import annotations

import os
import time
from typing import Any

import httpx

__all__ = ["ZVault"]

DEFAULT_BASE_URL = "https://api.zvault.cloud"
DEFAULT_TIMEOUT = 10
MAX_RETRIES = 2


class ZVaultError(Exception):
    pass


class ZVault:
    """Flask extension that loads ZVault secrets into app.config."""

    def __init__(self, app: Any = None, env: str = "production", **kwargs: Any):
        self._env = env
        self._kwargs = kwargs
        if app is not None:
            self.init_app(app)

    def init_app(self, app: Any) -> None:
        """Initialize the extension with a Flask app."""
        secrets = _fetch_secrets_sync(env=self._env, **self._kwargs)
        app.config.update(secrets)
        app.extensions["zvault"] = self
        self._secrets = secrets

    def get(self, key: str, default: str | None = None) -> str | None:
        return self._secrets.get(key, default)

    def all(self) -> dict[str, str]:
        return dict(self._secrets)


def _fetch_secrets_sync(
    env: str = "production",
    token: str | None = None,
    org_id: str | None = None,
    project_id: str | None = None,
    base_url: str | None = None,
) -> dict[str, str]:
    _token = token or os.getenv("ZVAULT_TOKEN", "")
    _org_id = org_id or os.getenv("ZVAULT_ORG_ID", "")
    _project_id = project_id or os.getenv("ZVAULT_PROJECT_ID", "")
    _base_url = (base_url or os.getenv("ZVAULT_URL", DEFAULT_BASE_URL)).rstrip("/")

    if not _token or not _org_id or not _project_id:
        return {}

    url = f"{_base_url}/v1/cloud/orgs/{_org_id}/projects/{_project_id}/envs/{env}/secrets"

    for i in range(MAX_RETRIES + 1):
        try:
            with httpx.Client(timeout=DEFAULT_TIMEOUT) as client:
                res = client.get(
                    url,
                    headers={
                        "Authorization": f"Bearer {_token}",
                        "User-Agent": "flask-zvault/0.1.0",
                    },
                )
                if res.is_success:
                    data = res.json()
                    return {s["key"]: s["value"] for s in data.get("secrets", [])}

                if res.status_code < 500 and res.status_code != 429:
                    break
        except Exception:
            pass

        if i < MAX_RETRIES:
            time.sleep(0.3 * (2 ** i))

    return {}
