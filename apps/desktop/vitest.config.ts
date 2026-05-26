import { resolve } from 'node:path'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    projects: [
      {
        test: {
          name: 'main',
          environment: 'node',
          include: ['src/main/**/*.test.ts', 'src/preload/**/*.test.ts'],
        },
      },
      {
        test: {
          name: 'renderer',
          environment: 'jsdom',
          setupFiles: ['./src/renderer/src/test/setup.ts'],
          include: ['src/renderer/**/*.test.{ts,tsx}'],
          fileParallelism: false,
          alias: {
            '@renderer': resolve('src/renderer/src'),
          },
        },
      },
    ],
  },
})
