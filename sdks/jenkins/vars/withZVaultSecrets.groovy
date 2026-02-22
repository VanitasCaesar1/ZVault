/**
 * Jenkins shared library step: withZVaultSecrets
 *
 * Usage:
 *   withZVaultSecrets(env: 'production') {
 *       sh 'npm test'
 *   }
 */
def call(Map config = [:], Closure body) {
    def envSlug = config.env ?: 'production'
    def orgId = config.orgId ?: env.ZVAULT_ORG_ID
    def projectId = config.projectId ?: env.ZVAULT_PROJECT_ID

    // Use ZVault CLI to fetch secrets as KEY=VALUE pairs
    def secretsOutput = sh(
        script: """
            zvault cloud pull \
                --env ${envSlug} \
                --org ${orgId} \
                --project ${projectId} \
                --format shell 2>/dev/null || echo ""
        """,
        returnStdout: true
    ).trim()

    if (secretsOutput.isEmpty()) {
        echo "[zvault] Warning: No secrets fetched for env '${envSlug}'"
        body()
        return
    }

    def envVars = secretsOutput.split('\n').findAll { it.contains('=') }.collect { it.trim() }
    echo "[zvault] Injecting ${envVars.size()} secrets from '${envSlug}'"

    withEnv(envVars) {
        body()
    }
}
