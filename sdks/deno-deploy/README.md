# ZVault + Deno Deploy

Inject secrets from ZVault Cloud into Deno Deploy edge functions.

## Usage

The Node.js SDK (`@zvault/sdk`) works with Deno via npm specifiers:

```typescript
// main.ts
import { ZVault } from "npm:@zvault/sdk";

const vault = new ZVault({
  token: Deno.env.get("ZVAULT_TOKEN")!,
});

// Fetch all secrets at startup
const secrets = await vault.getAll({ env: "production" });

Deno.serve((req) => {
  const dbUrl = secrets.DATABASE_URL;
  return new Response(`Connected to: ${dbUrl ? "✅" : "❌"}`);
});
```

## Setup

1. Set `ZVAULT_TOKEN` in Deno Deploy dashboard (Project → Settings → Environment Variables)

2. Deploy:
   ```bash
   deployctl deploy --project=my-app main.ts
   ```

## With Fresh Framework

```typescript
// fresh.config.ts
import { ZVault } from "npm:@zvault/sdk";

const vault = new ZVault({
  token: Deno.env.get("ZVAULT_TOKEN")!,
});

// Load secrets before server starts
export const secrets = await vault.getAll({ env: "production" });
```

```typescript
// routes/api/health.ts
import { secrets } from "../../fresh.config.ts";

export const handler = {
  GET() {
    return new Response(JSON.stringify({
      db: secrets.DATABASE_URL ? "connected" : "missing",
    }));
  },
};
```

## Environment Variables

Set in Deno Deploy dashboard:

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token |
