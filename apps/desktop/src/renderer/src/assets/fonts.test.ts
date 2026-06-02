import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const FONT_DIR = resolve('src/renderer/src/assets/fonts')

describe('bundled fonts', () => {
  it.each(['InterVariable.woff2', 'GeistMono-Variable.woff2'])(
    '%s is a valid WOFF2 asset',
    (filename) => {
      const font = readFileSync(resolve(FONT_DIR, filename))

      expect(font.subarray(0, 4).toString('ascii')).toBe('wOF2')
    },
  )
})
