"""Official ZVault SDK for Python."""

from zvault.client import ZVault
from zvault.errors import (
    ZVaultError,
    ZVaultAPIError,
    ZVaultConfigError,
    ZVaultNotFoundError,
    ZVaultAuthError,
    ZVaultTimeoutError,
)

__all__ = [
    "ZVault",
    "ZVaultError",
    "ZVaultAPIError",
    "ZVaultConfigError",
    "ZVaultNotFoundError",
    "ZVaultAuthError",
    "ZVaultTimeoutError",
]

__version__ = "0.1.0"
