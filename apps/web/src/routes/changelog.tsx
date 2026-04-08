import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { seo } from '~/lib/seo'
import changelogRaw from '../../content/changelog.md?raw'

const fetchChangelog = createServerFn({ method: 'GET' }).handler(async () => {
  const { renderToHtml, init } = await import('md4x')
  await init()
  const match = changelogRaw.match(/^---\n[\s\S]*?\n---\n([\s\S]*)$/)
  const markdown = match ? match[1] : changelogRaw
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
