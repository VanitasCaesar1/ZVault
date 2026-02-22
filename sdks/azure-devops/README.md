# ZVault Azure DevOps Task

Fetch secrets from ZVault Cloud and inject as pipeline variables.

## Usage

```yaml
# azure-pipelines.yml
steps:
  - task: ZVaultSecrets@0
    inputs:
      token: $(ZVAULT_TOKEN)
      orgId: $(ZVAULT_ORG_ID)
      projectId: $(ZVAULT_PROJECT_ID)
      env: production

  - script: |
      echo "Secrets are available as pipeline variables"
      npm test
    displayName: Run tests
```

## Setup

1. Add `ZVAULT_TOKEN`, `ZVAULT_ORG_ID`, `ZVAULT_PROJECT_ID` as secret pipeline variables
2. Add the ZVault task to your pipeline
3. All secrets are injected as secret pipeline variables

## License

MIT
