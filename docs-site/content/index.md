---
title: ZVault Documentation
description: The AI-native secrets manager built in Rust. Secure, fast, and developer-friendly.
navigation: false
---

::hero
---
actions:
  - label: Get Started
    to: /getting-started/introduction
    icon: i-lucide-arrow-right
    color: primary
  - label: View on GitHub
    to: https://github.com/ArcadeLabsInc/zvault
    icon: i-lucide-external-link
    target: _blank
    color: neutral
    variant: outline
---

#title
ZVault Documentation

#description
A secure, high-performance secrets manager built entirely in Rust. AES-256-GCM encryption, Shamir unseal, zero unsafe crypto.
::

::card-group
  ::card
  ---
  title: Quick Start
  icon: i-lucide-rocket
  to: /getting-started/quickstart
  ---
  Initialize your vault, store your first secret, and retrieve it — all in under a minute.
  ::

  ::card
  ---
  title: CLI Reference
  icon: i-lucide-terminal
  to: /cli/overview
  ---
  Full command reference for the `zvault` CLI. KV operations, import, status, and more.
  ::

  ::card
  ---
  title: AI Mode
  icon: i-lucide-sparkles
  to: /ai-mode/overview
  ---
  Use `zvault://` URIs in your IDE to reference secrets without exposing them. MCP server included.
  ::

  ::card
  ---
  title: Self-Hosting
  icon: i-lucide-server
  to: /self-hosting/docker
  ---
  Deploy ZVault on Railway, Docker, or bare metal. Production-ready with persistent storage.
  ::

  ::card
  ---
  title: API Reference
  icon: i-lucide-file-text
  to: /api/authentication
  ---
  RESTful API for KV secrets, system operations, and authentication.
  ::

  ::card
  ---
  title: Security
  icon: i-lucide-shield
  to: /security/architecture
  ---
  Encryption barrier, Shamir's Secret Sharing, audit logging, and threat model.
  ::
::
