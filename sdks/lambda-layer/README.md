# ZVault Lambda Layer

AWS Lambda extension that fetches secrets from ZVault Cloud at cold start.

## Setup

### 1. Create the layer

```bash
chmod +x bootstrap.sh
zip zvault-layer.zip bootstrap.sh
aws lambda publish-layer-version \
  --layer-name zvault \
  --zip-file fileb://zvault-layer.zip \
  --compatible-runtimes nodejs20.x python3.12
```

### 2. Attach to your function

```bash
aws lambda update-function-configuration \
  --function-name my-function \
  --layers arn:aws:lambda:us-east-1:123456789:layer:zvault:1 \
  --environment Variables="{ZVAULT_TOKEN=zvt_xxx,ZVAULT_ORG_ID=org_xxx,ZVAULT_PROJECT_ID=proj_xxx}"
```

### 3. Access secrets in your handler

```python
import os

def handler(event, context):
    db_url = os.environ["DATABASE_URL"]  # Injected by ZVault
    return {"statusCode": 200}
```

## How It Works

1. Lambda cold start triggers the extension
2. Extension fetches all secrets from ZVault Cloud API
3. Secrets are exported as environment variables
4. Your handler runs with secrets available in `process.env` / `os.environ`

## License

MIT
