import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'noti',
  description: 'A unified multi-channel notification CLI — built for AI agents & automation',

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo.svg' }],
  ],

  lastUpdated: true,
  cleanUrls: true,

  themeConfig: {
    logo: '/logo.svg',

    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Providers', link: '/providers/overview' },
      { text: 'Reference', link: '/reference/cli' },
      {
        text: 'v0.1.2',
        items: [
          { text: 'Changelog', link: 'https://github.com/loonghao/noti/blob/main/CHANGELOG.md' },
          { text: 'Releases', link: 'https://github.com/loonghao/noti/releases' },
        ],
      },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'What is noti?', link: '/guide/what-is-noti' },
            { text: 'Getting Started', link: '/guide/getting-started' },
          ],
        },
        {
          text: 'Usage',
          items: [
            { text: 'Sending Notifications', link: '/guide/sending-notifications' },
            { text: 'Configuration & Profiles', link: '/guide/configuration' },
            { text: 'AI Agent Integration', link: '/guide/agent-integration' },
          ],
        },
        {
          text: 'Development',
          items: [
            { text: 'Architecture', link: '/guide/architecture' },
            { text: 'Contributing', link: '/guide/contributing' },
          ],
        },
      ],
      '/providers/': [
        {
          text: 'Providers',
          items: [
            { text: 'Overview', link: '/providers/overview' },
            { text: 'Chat & IM', link: '/providers/chat' },
            { text: 'Push Notifications', link: '/providers/push' },
            { text: 'SMS & Messaging', link: '/providers/sms' },
            { text: 'Email', link: '/providers/email' },
            { text: 'Webhooks', link: '/providers/webhooks' },
            { text: 'Incident & Automation', link: '/providers/incident' },
            { text: 'IoT, Media & More', link: '/providers/iot-media' },
          ],
        },
      ],
      '/reference/': [
        {
          text: 'Reference',
          items: [
            { text: 'CLI Commands', link: '/reference/cli' },
            { text: 'URL Schemes', link: '/reference/url-schemes' },
            { text: 'Exit Codes', link: '/reference/exit-codes' },
            { text: 'Environment Variables', link: '/reference/environment-variables' },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/loonghao/noti' },
    ],

    editLink: {
      pattern: 'https://github.com/loonghao/noti/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2024-present Hal Long',
    },

    search: {
      provider: 'local',
    },
  },
})
