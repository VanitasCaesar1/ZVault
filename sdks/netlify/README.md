# @zvault/netlify-plugin

ZVault build plugin for Netlify — inject secrets at build time.

## Setup

```toml
# netlify.toml
[[plugins]]
package = "@zvault/netlify-plugin"
```

Set these in Netlify UI → Site settings → Environment variables:
- `ZVAULT_TOKEN`
- `ZVAULT_ORG_ID`
- `ZVAULT_PROJECT_ID`
- `ZVAULT_ENV` (optional, defaults to `production`)

## License

MIT
