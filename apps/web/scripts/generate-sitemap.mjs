import { readdir, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const docs = await readdir(join(root, 'content/docs'))
const routes = [
  '/',
  '/download',
  '/changelog',
  ...docs.filter((file) => file.endsWith('.md')).map((file) => `/docs/${file.slice(0, -3)}`),
]

const urls = routes
  .sort((a, b) => a.localeCompare(b))
  .map((route) => `  <url><loc>https://mdow.wania.app${route}</loc></url>`)
  .join('\n')
const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`

await writeFile(join(root, 'public/sitemap.xml'), sitemap)
