#!/usr/bin/env node
/**
 * One-shot: rebuild macOS .icns / Windows .ico / Linux .png from the master
 * brand SVG, applying the macOS icon-grid padding so the mark doesn't render
 * oversized in Dock / Alt-Tab / sidebar.
 *
 * Apple's macOS Big Sur+ icon grid: the visible mark fits a 824×824 region
 * inside a 1024×1024 canvas (~80.5% of width). We render the SVG into that
 * inner region and composite onto a transparent square canvas.
 *
 * Source: apps/desktop/src/renderer/src/assets/mdow-logo.svg
 * (re-sync that file from the canonical brand SVG before running this).
 *
 * Output: apps/desktop/resources/icon.{icns,ico,png}
 *
 * Usage: node apps/desktop/scripts/regenerate-icons.mjs
 */

import { Buffer } from 'node:buffer'
import { writeFileSync, mkdirSync, globSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const HERE = dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = resolve(HERE, '../../..')

// sharp is hoisted under pnpm's virtual store; resolve it dynamically rather
// than declaring it as a desktop dep (it's only used by this one-shot script).
const sharpDir = globSync(resolve(REPO_ROOT, 'node_modules/.pnpm/sharp@*/node_modules/sharp'))[0]
if (!sharpDir) throw new Error('sharp not found in pnpm store')
const sharp = require(sharpDir)

const SRC_SVG = resolve(REPO_ROOT, 'apps/desktop/src/renderer/src/assets/mdow-logo.svg')
const OUT_DIR = resolve(REPO_ROOT, 'apps/desktop/resources')
const SIZES = [16, 24, 32, 48, 64, 128, 256, 512, 1024]
// macOS HIG icon grid: content occupies ~824/1024 of the canvas
const CONTENT_RATIO = 824 / 1024

async function renderPadded(size) {
  const inner = Math.round(size * CONTENT_RATIO)
  const offset = Math.round((size - inner) / 2)
  const innerPng = await sharp(SRC_SVG, { density: 384 })
    .resize(inner, inner, { fit: 'contain' })
    .png()
    .toBuffer()
  return sharp({
    create: {
      width: size,
      height: size,
      channels: 4,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    },
  })
    .composite([{ input: innerPng, top: offset, left: offset }])
    .png()
    .toBuffer()
}

function buildIcns(buffersBySize) {
  // OSType → pixel size
  const types = [
    ['ic07', 128],
    ['ic08', 256],
    ['ic09', 512],
    ['ic10', 1024],
    ['ic11', 32],
    ['ic12', 64],
    ['ic13', 256],
    ['ic14', 512],
  ]
  const parts = []
  for (const [osType, size] of types) {
    const data = buffersBySize.get(size)
    if (!data) throw new Error(`missing ${size}px PNG for ${osType}`)
    const header = Buffer.alloc(8)
    header.write(osType, 0, 4, 'ascii')
    header.writeUInt32BE(data.length + 8, 4)
    parts.push(header, data)
  }
  const body = Buffer.concat(parts)
  const header = Buffer.alloc(8)
  header.write('icns', 0, 4, 'ascii')
  header.writeUInt32BE(body.length + 8, 4)
  return Buffer.concat([header, body])
}

function buildIco(buffersBySize) {
  // Windows .ico — up to 256 px per entry. Larger sizes are skipped.
  const sizes = [16, 24, 32, 48, 64, 128, 256]
  const entries = sizes.map((s) => ({ size: s, data: buffersBySize.get(s) }))
  const headerLen = 6 + entries.length * 16
  let offset = headerLen
  const dir = Buffer.alloc(headerLen)
  dir.writeUInt16LE(0, 0) // reserved
  dir.writeUInt16LE(1, 2) // type = ICO
  dir.writeUInt16LE(entries.length, 4)
  entries.forEach((e, i) => {
    const base = 6 + i * 16
    const dim = e.size === 256 ? 0 : e.size
    dir.writeUInt8(dim, base + 0)
    dir.writeUInt8(dim, base + 1)
    dir.writeUInt8(0, base + 2) // color palette
    dir.writeUInt8(0, base + 3) // reserved
    dir.writeUInt16LE(1, base + 4) // planes
    dir.writeUInt16LE(32, base + 6) // bpp
    dir.writeUInt32LE(e.data.length, base + 8)
    dir.writeUInt32LE(offset, base + 12)
    offset += e.data.length
  })
  return Buffer.concat([dir, ...entries.map((e) => e.data)])
}

async function main() {
  mkdirSync(OUT_DIR, { recursive: true })
  const pngBySize = new Map()
  for (const size of SIZES) {
    const buf = await renderPadded(size)
    pngBySize.set(size, buf)
  }
  writeFileSync(resolve(OUT_DIR, 'icon.png'), pngBySize.get(1024))
  writeFileSync(resolve(OUT_DIR, 'icon.icns'), buildIcns(pngBySize))
  writeFileSync(resolve(OUT_DIR, 'icon.ico'), buildIco(pngBySize))
  console.log('wrote', OUT_DIR + '/icon.{icns,ico,png}')
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
