/**
 * Linux font loading. GPUI's Linux text system builds its font database from fontconfig
 * (fontdb honors `FONTCONFIG_FILE`), and gpuix has no API to add fonts. So point fontconfig at
 * a config that keeps the system setup and adds the bundled fonts folder, without installing
 * anything into the user's home.
 *
 * Must run before `render()`: GPUI reads the fonts once when the app starts.
 */
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { dlopen, FFIType } from 'bun:ffi'

const xml = (value: string) =>
  value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')

export function fontconfigFile(fontsDir: string, base = '/etc/fonts/fonts.conf') {
  return [
    '<?xml version="1.0"?>',
    '<!DOCTYPE fontconfig SYSTEM "fonts.dtd">',
    '<fontconfig>',
    `  <include ignore_missing="yes">${xml(base)}</include>`,
    `  <dir>${xml(fontsDir)}</dir>`,
    '</fontconfig>',
    '',
  ].join('\n')
}

/**
 * Copy fonts embedded in the binary into a real folder. A compiled Bun binary serves them from
 * its virtual filesystem, which fontconfig cannot read. Unchanged files are left alone.
 */
export function extractFonts(fonts: Record<string, string>, dir: string) {
  mkdirSync(dir, { recursive: true })
  for (const [name, source] of Object.entries(fonts)) {
    const bytes = readFileSync(source)
    const dest = join(dir, name)
    let size = -1
    try {
      size = statSync(dest).size
    } catch {}
    if (size !== bytes.length) writeFileSync(dest, bytes)
  }
}

export function registerFontsLinux(fonts: Record<string, string>) {
  const cache = process.env.XDG_CACHE_HOME || join(homedir(), '.cache')
  const dir = join(cache, 'mdow-native')
  const fontsDir = join(dir, 'fonts')
  const path = join(dir, 'fonts.conf')
  try {
    extractFonts(fonts, fontsDir)
    // A relaunched app inherits our own FONTCONFIG_FILE; never include the file in itself.
    const inherited = process.env.FONTCONFIG_FILE
    writeFileSync(
      path,
      fontconfigFile(fontsDir, inherited && inherited !== path ? inherited : undefined),
    )
  } catch {
    return
  }
  // Bun's process.env writes don't reach the C environment the native addon reads.
  try {
    const libc = dlopen('libc.so.6', {
      setenv: { args: [FFIType.cstring, FFIType.cstring, FFIType.i32], returns: FFIType.i32 },
    })
    libc.symbols.setenv(Buffer.from('FONTCONFIG_FILE\0'), Buffer.from(`${path}\0`), 1)
    libc.close()
  } catch {
    // Non-glibc systems keep system fonts.
  }
}
