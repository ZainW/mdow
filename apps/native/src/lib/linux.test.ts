import { describe, expect, test } from 'bun:test'
import { mkdtempSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { extractFonts, fontconfigFile } from './linux'

describe('fontconfigFile', () => {
  test('keeps the system config and adds the fonts folder', () => {
    const xml = fontconfigFile('/tmp/fonts & more')
    expect(xml).toContain('<include ignore_missing="yes">/etc/fonts/fonts.conf</include>')
    expect(xml).toContain('<dir>/tmp/fonts &amp; more</dir>')
  })

  test('includes an inherited config instead of the default', () => {
    expect(fontconfigFile('/f', '/home/me/fonts.conf')).toContain('>/home/me/fonts.conf</include>')
  })
})

describe('extractFonts', () => {
  test('copies fonts and leaves unchanged files alone', () => {
    const src = mkdtempSync(join(tmpdir(), 'mdow-fonts-src-'))
    const dest = join(mkdtempSync(join(tmpdir(), 'mdow-fonts-dest-')), 'fonts')
    writeFileSync(join(src, 'a.ttf'), 'font-a')
    extractFonts({ 'A.ttf': join(src, 'a.ttf') }, dest)
    expect(readFileSync(join(dest, 'A.ttf'), 'utf8')).toBe('font-a')
    const first = statSync(join(dest, 'A.ttf')).mtimeMs
    extractFonts({ 'A.ttf': join(src, 'a.ttf') }, dest)
    expect(statSync(join(dest, 'A.ttf')).mtimeMs).toBe(first)
  })
})
