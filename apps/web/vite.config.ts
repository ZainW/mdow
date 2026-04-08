import { defineConfig } from 'vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import { cloudflare } from '@cloudflare/vite-plugin'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [
    cloudflare({ viteEnvironment: { name: 'ssr' } }),
    tanstackStart({
      prerender: {
        routes: ['/', '/download', '/changelog', '/docs', '/docs/*'],
        crawlLinks: true,
      },
    }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '~': resolve(__dirname, './src'),
    },
  },
  // Add the `unwasm` export condition to the SSR environment so md4x resolves
  // to its bundler-friendly variant (lib/wasm/unwasm.mjs) instead of the
  // default one that tries to readFile its own wasm via a relative URL.
  environments: {
    ssr: {
      resolve: {
        conditions: ['unwasm', 'workerd', 'worker', 'module', 'browser'],
      },
    },
  },
})
