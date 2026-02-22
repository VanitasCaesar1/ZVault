/**
 * ZVault Netlify Build Plugin
 *
 * Fetches secrets from ZVault Cloud at build time and injects as env vars.
 *
 * netlify.toml:
 *   [[plugins]]
 *   package = "@zvault/netlify-plugin"
 */

module.exports = {
  async onPreBuild({ utils }) {
    const token = process.env.ZVAULT_TOKEN;
    const orgId = process.env.ZVAULT_ORG_ID;
    const projectId = process.env.ZVAULT_PROJECT_ID;
    const env = process.env.ZVAULT_ENV || 'production';
    const baseUrl = (process.env.ZVAULT_URL || 'https://api.zvault.cloud').replace(/\/+$/, '');

    if (!token || !orgId || !projectId) {
      console.log('[zvault] Missing config — skipping secret injection');
      return;
    }

    const url = `${baseUrl}/v1/cloud/orgs/${orgId}/projects/${projectId}/envs/${env}/secrets`;

    try {
      const res = await fetch(url, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
          'User-Agent': '@zvault/netlify-plugin/0.1.0',
        },
      });

      if (!res.ok) {
        utils.build.failPlugin(`ZVault API returned HTTP ${res.status}`);
        return;
      }

      const data = await res.json();
      let count = 0;

      for (const s of data.secrets || []) {
        if (s.key && s.value && !process.env[s.key]) {
          process.env[s.key] = s.value;
          count++;
        }
      }

      console.log(`[zvault] Injected ${count} secrets from '${env}'`);
    } catch (err) {
      utils.build.failPlugin(`Failed to fetch secrets: ${err.message}`);
    }
  },
};
