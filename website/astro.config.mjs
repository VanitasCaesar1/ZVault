import { defineConfig } from 'astro/config';
import node from '@astrojs/node';

export default defineConfig({
  site: 'https://zvault.cloud',
  server: { port: 4400 },
  adapter: node({ mode: 'standalone' }),
});
