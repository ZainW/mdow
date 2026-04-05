import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { Comark } from '@comark/react'
import { getDoc, getAllDocs } from '~/lib/content'
import { DocsLayout } from '~/components/docs-layout'
import { DocsNav } from '~/components/docs-nav'
import { docsComponents } from '~/components/comark-components'
import { seo } from '~/lib/seo'

const fetchDoc = createServerFn({ method: 'GET' })
  .inputValidator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const [doc, allDocs] = await Promise.all([getDoc(slug), getAllDocs()])
    if (!doc) throw new Error(`Doc not found: ${slug}`)
    return { doc, allDocs }
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
  const { doc, allDocs } = Route.useLoaderData()

  return (
    <DocsLayout docs={allDocs} currentSlug={doc.meta.slug} headings={[]}>
      <Comark markdown={doc.raw} components={docsComponents} />
      <DocsNav docs={allDocs} currentSlug={doc.meta.slug} />
    </DocsLayout>
  )
}
