# django-zvault

ZVault integration for Django — load secrets in `settings.py`.

## Install

```bash
pip install django-zvault
```

## Quick Start

```python
# settings.py
from django_zvault import load_secrets

ZVAULT_SECRETS = load_secrets(env="production")

SECRET_KEY = ZVAULT_SECRETS.get("DJANGO_SECRET_KEY", "fallback")
DATABASES = {
    "default": {
        "ENGINE": "django.db.backends.postgresql",
        "HOST": ZVAULT_SECRETS.get("DB_HOST", "localhost"),
        "NAME": ZVAULT_SECRETS.get("DB_NAME", "mydb"),
        "USER": ZVAULT_SECRETS.get("DB_USER", "postgres"),
        "PASSWORD": ZVAULT_SECRETS.get("DB_PASSWORD", ""),
    }
}
```

## Environment Variables

```bash
ZVAULT_TOKEN=zvt_your_service_token
ZVAULT_ORG_ID=org_xxx
ZVAULT_PROJECT_ID=proj_xxx
```

## License

MIT
