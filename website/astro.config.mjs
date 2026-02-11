import { defineConfig } from 'astro/config';
import node from '@astrojs/node';
import tailwind from '@astrojs/tailwind';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://zvault.cloud',
  server: { port: 4400 },
  adapter: node({ mode: 'standalone' }),
  integrations: [tailwind(), sitemap()],
});
