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
      // Less JavaScript to parse and compile on every launch.
      minify: true,
      rollupOptions: {
        input: resolve('src/renderer/index.html'),
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
