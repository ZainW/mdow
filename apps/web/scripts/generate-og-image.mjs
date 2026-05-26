// Generate a 1200×630 Open Graph image for social sharing.
// Run: node apps/web/scripts/generate-og-image.mjs

import { chromium } from '../../desktop/node_modules/playwright/index.mjs'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const outPath = join(__dirname, '../public/og-image.png')
const screenshotPath = join(__dirname, '../public/screenshots/reading-dark.webp')

const screenshotB64 = readFileSync(screenshotPath).toString('base64')

const html = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      width: 1200px;
      height: 630px;
      font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
      background: radial-gradient(ellipse 80% 70% at 30% 0%, #2a2520 0%, #141414 55%);
      color: #f5f5f4;
      display: flex;
      align-items: center;
      padding: 56px 64px;
      gap: 48px;
      overflow: hidden;
    }
    .copy { flex: 1; min-width: 0; }
    .eyebrow {
      font-size: 14px;
      letter-spacing: 0.12em;
      text-transform: uppercase;
      color: #a8a29e;
      margin-bottom: 20px;
    }
    h1 {
      font-size: 52px;
      line-height: 1.05;
      letter-spacing: -0.03em;
      font-weight: 700;
      margin-bottom: 20px;
    }
    p {
      font-size: 22px;
      line-height: 1.45;
      color: #d6d3d1;
      max-width: 34ch;
    }
    .badge {
      display: inline-block;
      margin-top: 28px;
      padding: 10px 18px;
      border-radius: 999px;
      background: rgba(255,255,255,0.08);
      border: 1px solid rgba(255,255,255,0.12);
      font-size: 16px;
      color: #e7e5e4;
    }
    .shot {
      width: 520px;
      border-radius: 16px;
      overflow: hidden;
      box-shadow: 0 24px 80px rgba(0,0,0,0.45);
      border: 1px solid rgba(255,255,255,0.08);
      flex-shrink: 0;
    }
    .shot img { display: block; width: 100%; height: auto; }
  </style>
</head>
<body>
  <div class="copy">
    <div class="eyebrow">Mdow</div>
    <h1>A quiet place to read markdown</h1>
    <p>Beautiful rendering, syntax highlighting, and a calm reading experience for Mac, Windows, and Linux.</p>
    <div class="badge">Free download · mdow.app</div>
  </div>
  <div class="shot">
    <img src="data:image/webp;base64,${screenshotB64}" alt="" />
  </div>
</body>
</html>`

const browser = await chromium.launch()
const page = await browser.newPage({ viewport: { width: 1200, height: 630 } })
await page.setContent(html, { waitUntil: 'networkidle' })
await page.screenshot({ path: outPath, type: 'png' })
await browser.close()

console.log(`OG image saved to ${outPath}`)
