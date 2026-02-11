import { defineAppConfig } from 'nuxt/app'

export default defineAppConfig({
  seo: {
    title: 'ZVault Docs',
    description: 'Documentation for ZVault — the AI-native secrets manager built in Rust.',
  },

  header: {
    title: 'ZVault Docs',
    logo: {
      alt: 'ZVault',
      light: '/logo.svg',
      dark: '/logo.svg',
    },
  },

  ui: {
    colors: {
      primary: 'cyan',
      neutral: 'slate',
    },
  },

  toc: {
    title: 'On this page',
  },

  socials: {
    github: 'https://github.com/ArcadeLabsInc/zvault',
  },

  github: {
    url: 'https://github.com/ArcadeLabsInc/zvault',
    branch: 'main',
    rootDir: 'docs-site',
  },
})
