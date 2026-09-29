// Generate a 1200×630 Open Graph image for social sharing.
// Run: node apps/web/scripts/generate-og-image.mjs

import { chromium } from '../../desktop/node_modules/playwright/index.mjs'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const outPath = join(__dirname, '../public/og-image.png')
const b64 = (path) => readFileSync(join(__dirname, path)).toString('base64')

const screenshot = b64('../public/screenshots/reading-light.webp')
const logo = b64('../public/mdow-logo.svg')
const serif = b64('../src/assets/fonts/Newsreader-Variable.woff2')
const serifItalic = b64('../src/assets/fonts/Newsreader-Italic-Variable.woff2')
const sans = b64('../src/assets/fonts/InterVariable.woff2')

const html = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    @font-face { font-family: Newsreader; src: url(data:font/woff2;base64,${serif}); font-weight: 200 800; }
    @font-face { font-family: Newsreader; font-style: italic; src: url(data:font/woff2;base64,${serifItalic}); font-weight: 200 800; }
    @font-face { font-family: Inter; src: url(data:font/woff2;base64,${sans}); font-weight: 400 700; }
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      width: 1200px;
      height: 630px;
      overflow: hidden;
      position: relative;
      font-family: Inter, sans-serif;
      color: #26231f;
      background:
        radial-gradient(ellipse 60% 70% at 20% 0%, rgba(214, 120, 70, 0.08), transparent 70%),
        #faf7f2;
    }
    .lines {
      position: absolute;
      inset: 0;
      background: repeating-linear-gradient(to bottom, transparent 0, transparent 31px, rgba(38, 35, 31, 0.05) 31px, rgba(38, 35, 31, 0.05) 32px);
      -webkit-mask-image: linear-gradient(to right, black, transparent 55%);
    }
    .copy { position: absolute; left: 72px; top: 78px; width: 520px; }
    .brand { display: flex; align-items: center; gap: 14px; font-size: 26px; font-weight: 600; letter-spacing: -0.01em; }
    .brand img { width: 44px; height: 44px; border-radius: 10px; box-shadow: 0 0 0 1px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.08); }
    h1 {
      margin-top: 64px;
      font-family: Newsreader, serif;
      font-weight: 440;
      font-size: 76px;
      line-height: 0.98;
      letter-spacing: -0.02em;
    }
    h1 em { font-style: italic; }
    p { margin-top: 26px; font-size: 23px; line-height: 1.45; color: #6b645c; max-width: 25ch; }
    .url { position: absolute; left: 72px; bottom: 60px; font-size: 18px; color: #8a8279; }
    .shot {
      position: absolute;
      left: 640px;
      top: 92px;
      width: 820px;
      border-radius: 14px;
      overflow: hidden;
      background: #fff;
      box-shadow:
        0 0 0 1px rgba(38, 35, 31, 0.1),
        0 12px 24px -8px rgba(60, 40, 20, 0.12),
        0 40px 80px -24px rgba(60, 40, 20, 0.22);
    }
    .shot img { display: block; width: 100%; height: auto; }
    .lights { position: absolute; left: 11px; top: 17px; display: flex; gap: 4px; }
    .lights span { width: 6px; height: 6px; border-radius: 50%; }
  </style>
</head>
<body>
  <div class="lines"></div>
  <div class="copy">
    <div class="brand"><img src="data:image/svg+xml;base64,${logo}" alt="" />Mdow</div>
    <h1>A quiet place to read <em>markdown</em>.</h1>
    <p>A fast, focused reader for your notes, docs, and READMEs.</p>
  </div>
  <div class="url">mdow.wania.app · Free for macOS, Windows &amp; Linux</div>
  <div class="shot">
    <img src="data:image/webp;base64,${screenshot}" alt="" />
    <div class="lights"><span style="background:#ff5f57"></span><span style="background:#febc2e"></span><span style="background:#28c840"></span></div>
  </div>
</body>
</html>`

const browser = await chromium.launch()
const page = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 })
await page.setContent(html, { waitUntil: 'networkidle' })
await page.evaluate(() => document.fonts.ready)
await page.screenshot({ path: outPath, type: 'png' })
await browser.close()

console.log(`OG image saved to ${outPath}`)
