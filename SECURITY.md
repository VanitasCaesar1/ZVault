# Security Policy

## Reporting a Vulnerability

ZVault takes security seriously. If you discover a security vulnerability, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

### How to Report

Email: **security@zvault.cloud**

Include:
- Description of the vulnerability
- Steps to reproduce
- Impact assessment
- Suggested fix (if any)

### Response Timeline

| Stage | Timeline |
|-------|----------|
| Acknowledgment | Within 48 hours |
| Initial assessment | Within 5 business days |
| Fix development | Depends on severity |
| Public disclosure | After fix is released |

### Severity Levels

- **Critical**: Key material exposure, authentication bypass, remote code execution
- **High**: Privilege escalation, audit log bypass, encryption weakness
- **Medium**: Information disclosure, denial of service
- **Low**: Minor issues, hardening improvements

### What Qualifies

- Cryptographic weaknesses in the barrier, seal, or transit engines
- Authentication or authorization bypass
- Key material leakage (memory, logs, core dumps, swap)
- Audit log tampering or bypass
- Timing side-channels in token comparison
- MCP server exposing secret values (it should never return actual values)

### What Does NOT Qualify

- Vulnerabilities in dependencies (report upstream, but let us know)
- Denial of service via resource exhaustion (we're aware single-node has limits)
- Social engineering
- Issues requiring physical access to the host machine
- Bugs in the dashboard UI that don't affect security

### Safe Harbor

We will not pursue legal action against researchers who:
- Report vulnerabilities responsibly via the process above
- Do not access or modify other users' data
- Do not disrupt the service
- Allow reasonable time for a fix before disclosure

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅ Current |

## Security Design

See [docs/DESIGN.md](docs/DESIGN.md) for the full security architecture.
