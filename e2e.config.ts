import type { E2EConfig } from 'e2e'
import { web } from '@e2e-dev/web'
import { chatgpt } from 'e2e/oauth/chatgpt'

const port = 4317

export default {
  // Only agent.* steps use the model: sign in once with `pnpm exec e2e login openai`.
  agents: {
    default: {
      model: chatgpt('gpt-6-luna'),
      system: 'You are a thorough QA agent. Verify every outcome.',
      context:
        'This is the marketing site and docs for Mdow, a desktop markdown viewer for macOS, Windows, and Linux.',
    },
  },
  targets: [
    {
      name: 'web',
      engine: web(),
      app: {
        url: process.env.APP_URL ?? `http://localhost:${port}`,
        command: process.env.APP_URL
          ? undefined
          : {
              executable: 'pnpm',
              args: ['--filter', 'web', 'exec', 'vite', 'dev', '--port', `${port}`, '--strictPort'],
              log: '.e2e/logs/web.log',
            },
      },
    },
  ],
} satisfies E2EConfig
