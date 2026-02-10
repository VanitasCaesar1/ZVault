---
title: Installation
description: Install ZVault on macOS, Linux, or Windows.
sidebar:
  order: 2
---

## Quick Install (macOS / Linux)

```bash
curl -fsSL https://zvault.cloud/install.sh | sh
```

## Homebrew (macOS)

```bash
brew install zvault
```

## Cargo (from source)

```bash
cargo install zvault
```

## Docker

```bash
docker run -d --name zvault \
  -p 8200:8200 \
  -v zvault-data:/data \
  ghcr.io/nicosalm/zvault:latest
```

## Verify Installation

```bash
zvault --version
# zvault 0.1.0
```

## System Requirements

- **OS**: macOS (arm64/x86_64), Linux (x86_64/arm64), Windows (x86_64)
- **Memory**: 64MB minimum, 256MB recommended
- **Disk**: 50MB for binary + storage space for secrets
- **Network**: Only needed for team/cloud features (local vault works offline)
