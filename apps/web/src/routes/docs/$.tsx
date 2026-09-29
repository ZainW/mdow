import { createFileRoute, notFound } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { useRef } from 'react'
import { createPortal } from 'react-dom'
import { getDoc, getAllDocs, getDocBody, getSearchEntries } from '~/lib/content'
import { extractHeadings } from '~/lib/extract-headings'
import { DocsLayout } from '~/components/docs-layout'
import { DocsNav } from '~/components/docs-nav'
import { DocsCopyMarkdown } from '~/components/docs-copy-markdown'
import { CopyButton } from '~/components/copy-button'
import { useCodeBlockCopy } from '~/hooks/use-code-block-copy'
import { canonical, seo } from '~/lib/seo'

const fetchDoc = createServerFn({ method: 'GET' })
  .validator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const [doc, allDocs, searchEntries] = await Promise.all([
      getDoc(slug),
      getAllDocs(),
      getSearchEntries(),
    ])
    if (!doc) throw notFound()
    const headings = extractHeadings(doc.html).map((h) => ({
      id: h.id,
      text: h.text,
      level: h.level,
    }))
    const markdown = getDocBody(slug) ?? ''
    return { doc, allDocs, searchEntries, headings, markdown }
  })

export const Route = createFileRoute('/docs/$')({
  loader: async ({ params }) => {
    const slug = params._splat || 'getting-started'
    return fetchDoc({ data: slug })
  },
  head: ({ loaderData }) => ({
    meta: loaderData
      ? seo({
          title: `${loaderData.doc.meta.title} — Mdow Docs`,
          description: loaderData.doc.meta.description,
          path: `/docs/${loaderData.doc.meta.slug}`,
        })
      : [],
    links: loaderData ? [canonical(`/docs/${loaderData.doc.meta.slug}`)] : [],
  }),
  component: DocPage,
})

function DocPage() {
  const { doc, allDocs, searchEntries, headings, markdown } = Route.useLoaderData()
  const articleRef = useRef<HTMLDivElement>(null)
  const codeBlocks = useCodeBlockCopy(articleRef, doc.meta.slug)

  return (
    <DocsLayout
      docs={allDocs}
      searchEntries={searchEntries}
      currentSlug={doc.meta.slug}
      headings={headings}
    >
      <article className="mx-auto max-w-[44rem]">
        <div className="mb-8 flex items-center justify-between gap-4">
          <p className="text-sm text-muted-foreground">{doc.meta.category}</p>
          <DocsCopyMarkdown markdown={markdown} slug={doc.meta.slug} />
        </div>
        {/* Trusted: rendered from our own .md files by md4x on the server. */}
        <div
          ref={articleRef}
          className="prose max-w-none"
          dangerouslySetInnerHTML={{ __html: doc.html }}
        />
        {codeBlocks.map((target, i) =>
          createPortal(<CopyButton value={target.code} />, target.host, `copy-${i}`),
        )}
        <DocsNav docs={allDocs} currentSlug={doc.meta.slug} />
      </article>
    </DocsLayout>
  )
}
