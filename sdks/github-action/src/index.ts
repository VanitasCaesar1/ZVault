import * as core from '@actions/core';

interface SecretKey {
  key: string;
  version: number;
  comment: string;
  updated_at: string;
}

interface SecretEntry {
  key: string;
  value: string;
  version: number;
  comment: string;
  created_at: string;
  updated_at: string;
}

async function fetchApi<T>(
  baseUrl: string,
  token: string,
  path: string,
): Promise<T> {
  const url = `${baseUrl}/v1/cloud${path}`;
  const res = await fetch(url, {
    method: 'GET',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      'User-Agent': 'zvault-github-action/0.1.0',
    },
  });

  if (!res.ok) {
    const body = await res.json().catch(() => null) as { error?: { message?: string } } | null;
    const msg = body?.error?.message ?? `HTTP ${res.status}`;
    throw new Error(`ZVault API error: ${msg} (${res.status})`);
  }

  return res.json() as Promise<T>;
}

async function run(): Promise<void> {
  try {
    const token = core.getInput('token', { required: true });
    const orgId = core.getInput('org-id', { required: true });
    const projectId = core.getInput('project-id', { required: true });
    const env = core.getInput('env') || 'production';
    const baseUrl = (core.getInput('url') || 'https://api.zvault.cloud').replace(/\/+$/, '');
    const keysFilter = core.getInput('keys');
    const exportEnv = core.getInput('export-env') !== 'false';
    const mask = core.getInput('mask') !== 'false';

    core.info(`Fetching secrets from ZVault Cloud (env: ${env})`);

    // Fetch key list
    const basePath = `/orgs/${orgId}/projects/${projectId}/envs/${env}/secrets`;
    const { keys } = await fetchApi<{ keys: SecretKey[] }>(baseUrl, token, basePath);

    // Filter keys if specified
    let targetKeys = keys.map((k) => k.key);
    if (keysFilter) {
      const wanted = new Set(keysFilter.split(',').map((k) => k.trim()));
      targetKeys = targetKeys.filter((k) => wanted.has(k));
    }

    core.info(`Found ${targetKeys.length} secrets to inject`);

    // Fetch each secret value
    let count = 0;
    for (const key of targetKeys) {
      const { secret } = await fetchApi<{ secret: SecretEntry }>(
        baseUrl,
        token,
        `${basePath}/${encodeURIComponent(key)}`,
      );

      // Mask the value in logs
      if (mask) {
        core.setSecret(secret.value);
      }

      // Export as environment variable
      if (exportEnv) {
        core.exportVariable(key, secret.value);
      }

      // Also set as output (masked)
      core.setOutput(key, secret.value);
      count++;
    }

    core.setOutput('count', count.toString());
    core.info(`Successfully injected ${count} secrets`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    core.setFailed(message);
  }
}

run();
