# zvault-storage

Storage backend abstraction for [ZVault](https://zvault.cloud) — the AI-native secrets manager.

This crate defines the `StorageBackend` trait and provides three implementations:

- `RocksDbBackend` — production default, powered by RocksDB (feature: `rocksdb-backend`)
- `RedbBackend` — pure-Rust alternative, powered by redb (feature: `redb-backend`)
- `MemoryBackend` — in-memory, for testing

The storage layer only ever sees ciphertext. All encryption/decryption happens in the barrier layer (`zvault-core`).

## Usage

```rust
use zvault_storage::{StorageBackend, MemoryBackend};

let storage = MemoryBackend::new();
storage.put("sys/config", b"encrypted-data").await?;
let data = storage.get("sys/config").await?;
```

## Part of ZVault

```
CLI / MCP Server / Web UI
        │
   ┌────▼────┐
   │ Barrier  │  ← zvault-core (encrypt/decrypt)
   └────┬────┘
   ┌────▼────┐
   │ Storage  │  ← this crate (ciphertext only)
   └─────────┘
```

Install the full CLI: `cargo install zvault-cli`

[Website](https://zvault.cloud) · [Docs](https://docs.zvault.cloud) · [GitHub](https://github.com/VanitasCaesar1/zvault)
