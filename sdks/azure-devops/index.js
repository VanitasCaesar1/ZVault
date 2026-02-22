#!/usr/bin/env node

/**
 * Azure DevOps Task: ZVault Inject Secrets
 *
 * Fetches secrets from ZVault Cloud and sets them as pipeline variables.
 */

const https = require('https');
const url = require('url');

const token = process.env.INPUT_TOKEN || '';
const orgId = process.env.INPUT_ORGID || '';
const projectId = process.env.INPUT_PROJECTID || '';
const env = process.env.INPUT_ENV || 'production';
const baseUrl = process.env.ZVAULT_URL || 'https://api.zvault.cloud';

if (!token || !orgId || !projectId) {
  console.log('##vso[task.logissue type=error]Missing required inputs: token, orgId, projectId');
  process.exit(1);
}

const secretsUrl = `${baseUrl}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${env}/secrets`;
const parsed = new url.URL(secretsUrl);

const options = {
  hostname: parsed.hostname,
  port: parsed.port || 443,
  path: parsed.pathname,
  method: 'GET',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
    'User-Agent': 'zvault-azure-devops/0.1.0',
  },
};

const req = https.request(options, (res) => {
  let body = '';
  res.on('data', (chunk) => { body += chunk; });
  res.on('end', () => {
    if (res.statusCode < 200 || res.statusCode >= 300) {
      console.log(`##vso[task.logissue type=error]ZVault API returned HTTP ${res.statusCode}`);
      process.exit(1);
    }

    try {
      const data = JSON.parse(body);
      const secrets = data.secrets || [];
      let count = 0;

      for (const s of secrets) {
        if (s.key && s.value) {
          // Set as pipeline variable (secret)
          console.log(`##vso[task.setvariable variable=${s.key};issecret=true]${s.value}`);
          count++;
        }
      }

      console.log(`[zvault] Injected ${count} secrets from '${env}'`);
    } catch (e) {
      console.log(`##vso[task.logissue type=error]Failed to parse response: ${e.message}`);
      process.exit(1);
    }
  });
});

req.on('error', (e) => {
  console.log(`##vso[task.logissue type=error]Request failed: ${e.message}`);
  process.exit(1);
});

req.end();
