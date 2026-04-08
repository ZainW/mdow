import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { seo } from '~/lib/seo'

const fetchChangelog = createServerFn({ method: 'GET' }).handler(async () => {
  const { readFile } = await import('node:fs/promises')
  const { join } = await import('node:path')
  const { renderToHtml, init } = await import('md4x')
  await init()
  const raw = await readFile(join(process.cwd(), 'content', 'changelog.md'), 'utf-8')
  const match = raw.match(/^---\n[\s\S]*?\n---\n([\s\S]*)$/)
  const markdown = match ? match[1] : raw
  return renderToHtml(markdown)
})

export const Route = createFileRoute('/changelog')({
  loader: () => fetchChangelog(),
  head: () => ({
    meta: seo({
      title: 'Changelog — Mdow',
      description: "What's new in Mdow.",
    }),
  }),
  component: ChangelogPage,
})

function ChangelogPage() {
  const html = Route.useLoaderData()

  return (
    <div className="mx-auto max-w-3xl px-6 py-16">
      {/* Content is trusted — rendered from our own changelog.md by md4x server-side */}
      <article
        className="prose prose-neutral dark:prose-invert max-w-none"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  )
}
