import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { WINDOW_COLORS } from './theme-colors'

const currentDir = dirname(fileURLToPath(import.meta.url))
const rendererStyles = readFileSync(
  resolve(currentDir, '../renderer/src/assets/styles/index.css'),
  'utf8',
)

describe('window theme colors', () => {
  it('keeps light Electron chrome aligned with the warm paper renderer palette', () => {
    expect(rendererStyles).toContain('--background: oklch(0.98 0.005 70);')
    expect(rendererStyles).toContain('--foreground: oklch(0.13 0.02 50);')
    expect(WINDOW_COLORS.light).toEqual({
      background: '#fbf8f5',
      foreground: '#0e0502',
    })
  })
})
