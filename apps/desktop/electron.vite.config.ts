import { resolve } from 'path'
import { defineConfig } from 'electron-vite'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  main: {},
  preload: {},
  renderer: {
    resolve: {
      alias: {
        '@renderer': resolve('src/renderer/src'),
      },
    },
    plugins: [tailwindcss(), react()],
    build: {
      rollupOptions: {
        input: resolve('src/renderer/index.html'),
        output: {
          manualChunks(id: string) {
            if (id.includes('/components/SettingsDialog')) return 'ui-settings'
            if (id.includes('/components/ShortcutsDialog')) return 'ui-shortcuts'
            if (id.includes('/components/CommandPalette')) return 'ui-command'
            if (id.includes('/components/companion/')) return 'companion'
            return undefined
          },
        },
      },
    },
    server: {
      watch: {
        ignored: [
          '**/node_modules/**',
          '**/.git/**',
          '**/out/**',
          '**/dist/**',
          '**/.turbo/**',
          '**/coverage/**',
          '**/resources/ai/**',
        ],
      },
    },
  },
})
