import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://docs.zvault.cloud',
  integrations: [
    starlight({
      title: 'ZVault Docs',
      description: 'Documentation for ZVault — the AI-native secrets manager.',
      // logo: { src: './src/assets/logo.svg', alt: 'ZVault', replacesTitle: false },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/nicosalm/zvault' },
      ],
      sidebar: [
        {
          label: 'Getting Started',
          autogenerate: { directory: 'getting-started' },
        },
        {
          label: 'CLI Reference',
          autogenerate: { directory: 'cli' },
        },
        {
          label: 'AI Mode (Pro)',
          autogenerate: { directory: 'ai-mode' },
        },
        {
          label: 'API Reference',
          autogenerate: { directory: 'api' },
        },
        {
          label: 'Self-Hosting',
          autogenerate: { directory: 'self-hosting' },
        },
        {
          label: 'Security',
          autogenerate: { directory: 'security' },
        },
      ],
      customCss: ['./src/styles/custom.css'],
      editLink: { baseUrl: 'https://github.com/nicosalm/zvault/edit/main/docs-site/' },
    }),
  ],
});
