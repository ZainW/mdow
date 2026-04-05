import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { Comark } from '@comark/react'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { seo } from '~/lib/seo'

const fetchChangelog = createServerFn({ method: 'GET' }).handler(async () => {
  const raw = await readFile(join(process.cwd(), 'content', 'changelog.md'), 'utf-8')
  const match = raw.match(/^---\n[\s\S]*?\n---\n([\s\S]*)$/)
  return match ? match[1] : raw
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
  const content = Route.useLoaderData()

  return (
    <div className="mx-auto max-w-3xl px-6 py-16">
      <article className="prose prose-neutral dark:prose-invert max-w-none">
        <Comark markdown={content} />
      </article>
    </div>
  )
}
