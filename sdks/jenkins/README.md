# ZVault Jenkins Plugin

Jenkins Credentials Provider + Pipeline step for ZVault Cloud secrets.

## Pipeline Usage

```groovy
pipeline {
    agent any

    environment {
        ZVAULT_TOKEN = credentials('zvault-token')
    }

    stages {
        stage('Build') {
            steps {
                withZVaultSecrets(env: 'production') {
                    sh 'echo "DB_URL is set: $DATABASE_URL"'
                    sh 'npm test'
                }
            }
        }
    }
}
```

## Shared Library

Add to your Jenkins shared library:

```groovy
// vars/withZVaultSecrets.groovy
def call(Map config = [:], Closure body) {
    def env = config.env ?: 'production'
    def token = config.token ?: env.ZVAULT_TOKEN

    def secrets = sh(
        script: "zvault cloud pull --env ${env} --format shell",
        returnStdout: true
    ).trim()

    withEnv(secrets.split('\n').collect { it }) {
        body()
    }
}
```

## Credentials Provider

Install the ZVault CLI on your Jenkins agents:

```bash
curl -fsSL https://zvault.cloud/install.sh | sh
```

Then configure credentials in Jenkins:
1. Go to Manage Jenkins → Credentials
2. Add a "Secret text" credential with your `ZVAULT_TOKEN`
3. Reference it in pipelines via `credentials('zvault-token')`

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ZVAULT_TOKEN` | Service token |
| `ZVAULT_ORG_ID` | Organization ID |
| `ZVAULT_PROJECT_ID` | Project ID |

## License

MIT
