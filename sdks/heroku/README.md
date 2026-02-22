# ZVault + Heroku

Inject secrets from ZVault Cloud into Heroku dynos at boot.

## Buildpack

Add the ZVault buildpack to your Heroku app:

```bash
heroku buildpacks:add https://github.com/ArcadeLabsInc/zvault-heroku-buildpack
```

This installs the ZVault CLI during slug compilation.

## Procfile

Use `zvault run` as your process wrapper:

```procfile
web: zvault run --env production -- node server.js
worker: zvault run --env production -- node worker.js
```

## Setup

1. Set your ZVault token:
   ```bash
   heroku config:set ZVAULT_TOKEN=zvt_xxx
   ```

2. Add the buildpack:
   ```bash
   heroku buildpacks:add https://github.com/ArcadeLabsInc/zvault-heroku-buildpack
   ```

3. Deploy:
   ```bash
   git push heroku main
   ```

## Alternative: Profile.d Script

Create `.profile.d/zvault.sh` in your repo:

```bash
#!/usr/bin/env bash
# .profile.d/zvault.sh — runs at dyno boot
if [ -n "$ZVAULT_TOKEN" ]; then
  export PATH="$HOME/.zvault/bin:$PATH"
  eval "$(zvault cloud pull --env production --format env)"
fi
```

## Alternative: Runtime SDK

```typescript
import { ZVault } from '@zvault/sdk';

const vault = new ZVault({ token: process.env.ZVAULT_TOKEN });
const secrets = await vault.getAll({ env: 'production' });
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token |
