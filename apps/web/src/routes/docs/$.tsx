import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { useRef } from 'react'
import { createPortal } from 'react-dom'
import { getDoc, getAllDocs } from '~/lib/content'
import { extractHeadings } from '~/lib/extract-headings'
import { DocsLayout } from '~/components/docs-layout'
import { DocsNav } from '~/components/docs-nav'
import { CopyButton } from '~/components/copy-button'
import { useCodeBlockCopy } from '~/hooks/use-code-block-copy'
import { seo } from '~/lib/seo'

const fetchDoc = createServerFn({ method: 'GET' })
  .inputValidator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const [doc, allDocs] = await Promise.all([getDoc(slug), getAllDocs()])
    if (!doc) throw new Error(`Doc not found: ${slug}`)
    const headings = extractHeadings(doc.html).map((h) => ({
      id: h.id,
      text: h.text,
      level: h.level,
    }))
    return { doc, allDocs, headings }
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
        })
      : [],
  }),
  component: DocPage,
})

function DocPage() {
  const { doc, allDocs, headings } = Route.useLoaderData()
  const articleRef = useRef<HTMLDivElement>(null)
  const codeBlocks = useCodeBlockCopy(articleRef, doc.meta.slug)

  // Content is trusted — rendered from our own .md files by md4x server-side
  return (
    <DocsLayout docs={allDocs} currentSlug={doc.meta.slug} headings={headings}>
      <div ref={articleRef} dangerouslySetInnerHTML={{ __html: doc.html }} />
      {codeBlocks.map((target, i) =>
        createPortal(<CopyButton value={target.code} />, target.host, `copy-${i}`),
      )}
      <DocsNav docs={allDocs} currentSlug={doc.meta.slug} />
    </DocsLayout>
  )
}
