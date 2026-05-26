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
      <div className="not-prose mb-8 flex items-center justify-between gap-4">
        <p className="text-sm text-muted-foreground">Follow updates via RSS</p>
        <a
          href="/changelog/rss.xml"
          className="text-sm text-primary underline underline-offset-2 hover:text-primary/80"
        >
          Subscribe to RSS
        </a>
      </div>
      <article
        className="prose prose-neutral dark:prose-invert max-w-none"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  )
}
