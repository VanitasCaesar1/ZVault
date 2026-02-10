# zvault

**Stop leaking secrets to LLMs.** The AI-native secrets manager.

This is the npm wrapper for the [ZVault](https://zvault.cloud) CLI. It downloads the correct platform binary on install.

## Usage

```bash
# One-shot
npx zvault import .env
npx zvault run -- npm run dev

# Or install globally
npm install -g zvault
zvault import .env
```

## What is ZVault?

ZVault replaces your `.env` file with `zvault://` references that are safe for AI coding tools to read. Your secrets stay encrypted locally — Cursor, Copilot, and Kiro never see real values.

```bash
zvault import .env        # Encrypt secrets, generate references
zvault run -- npm run dev # Inject real values at runtime
zvault setup cursor       # Connect AI via MCP (Pro)
```

## Links

- [Website](https://zvault.cloud)
- [Documentation](https://docs.zvault.cloud)
- [GitHub](https://github.com/zvault/zvault)

## License

MIT OR Apache-2.0
