# ZVault + Supabase Edge Functions

Inject secrets from ZVault Cloud into Supabase Edge Functions.

## Usage

```typescript
// supabase/functions/my-function/index.ts
import { ZVault } from "npm:@zvault/sdk";

const vault = new ZVault({
  token: Deno.env.get("ZVAULT_TOKEN")!,
});

Deno.serve(async (req) => {
  // Fetch secrets (cached in-memory after first call)
  const secrets = await vault.getAll({ env: "production" });

  const stripe = secrets.STRIPE_SECRET_KEY;
  // ... use secrets

  return new Response(JSON.stringify({ ok: true }), {
    headers: { "Content-Type": "application/json" },
  });
});
```

## Setup

1. Set the ZVault token as a Supabase secret:
   ```bash
   supabase secrets set ZVAULT_TOKEN=zvt_xxx
   ```

2. Deploy your function:
   ```bash
   supabase functions deploy my-function
   ```

## With Supabase Client

```typescript
import { createClient } from "npm:@supabase/supabase-js";
import { ZVault } from "npm:@zvault/sdk";

const vault = new ZVault({
  token: Deno.env.get("ZVAULT_TOKEN")!,
});

const secrets = await vault.getAll({ env: "production" });

// Use ZVault-managed Supabase credentials
const supabase = createClient(
  secrets.SUPABASE_URL,
  secrets.SUPABASE_SERVICE_KEY,
);
```

## Environment Variables

Set via `supabase secrets set`:

| Variable | Required | Description |
|----------|----------|-------------|
| `ZVAULT_TOKEN` | Yes | Service token |
