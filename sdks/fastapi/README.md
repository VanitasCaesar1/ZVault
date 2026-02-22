# fastapi-zvault

ZVault integration for FastAPI — dependency injection for secrets.

## Install

```bash
pip install fastapi-zvault
```

## Quick Start

```python
from fastapi import FastAPI, Depends
from fastapi_zvault import get_secret, zvault_lifespan

app = FastAPI(lifespan=zvault_lifespan)

@app.get("/")
async def root(db_url: str = Depends(get_secret("DATABASE_URL"))):
    return {"db_connected": bool(db_url)}
```

## Environment Variables

```bash
ZVAULT_TOKEN=zvt_your_service_token
ZVAULT_ORG_ID=org_xxx
ZVAULT_PROJECT_ID=proj_xxx
ZVAULT_ENV=production
```

## License

MIT
