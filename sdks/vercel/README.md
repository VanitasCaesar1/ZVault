# ZVault for Vercel

Build-time secret injection for Vercel deployments.

## Setup

1. Add `ZVAULT_TOKEN`, `ZVAULT_ORG_ID`, `ZVAULT_PROJECT_ID` to your Vercel project's environment variables
2. Add the build script to your `package.json`:

```json
{
  "scripts": {
    "prebuild": "npx @zvault/next inject || true",
    "build": "next build"
  }
}
```

Or use the `@zvault/next` plugin directly in `next.config.mjs`:

```js
import { withZVault } from '@zvault/next/plugin';

export default withZVault({
  env: process.env.VERCEL_ENV || 'production',
  publicKeys: ['NEXT_PUBLIC_STRIPE_KEY'],
})({
  reactStrictMode: true,
});
```

## vercel.json

```json
{
  "build": {
    "env": {
      "ZVAULT_TOKEN": "@zvault-token",
      "ZVAULT_ORG_ID": "@zvault-org-id",
      "ZVAULT_PROJECT_ID": "@zvault-project-id"
    }
  }
}
```

## Environment Mapping

| Vercel Env | ZVault Env |
|------------|------------|
| `production` | `production` |
| `preview` | `staging` |
| `development` | `development` |

## License

MIT
