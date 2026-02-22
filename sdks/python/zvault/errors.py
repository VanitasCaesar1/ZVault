"""ZVault SDK error types."""


class ZVaultError(Exception):
    """Base error for all ZVault SDK errors."""


class ZVaultAPIError(ZVaultError):
    """Raised when the API returns an HTTP error."""

    def __init__(self, status_code: int, message: str) -> None:
        self.status_code = status_code
        self.message = message
        super().__init__(f"ZVault API error {status_code}: {message}")


class ZVaultConfigError(ZVaultError):
    """Raised when required configuration is missing."""


class ZVaultNotFoundError(ZVaultError):
    """Raised when a secret is not found."""

    def __init__(self, key: str, env: str) -> None:
        self.key = key
        self.env = env
        super().__init__(f'Secret "{key}" not found in environment "{env}"')


class ZVaultAuthError(ZVaultError):
    """Raised when authentication fails (401/403)."""


class ZVaultTimeoutError(ZVaultError):
    """Raised when a request times out."""
