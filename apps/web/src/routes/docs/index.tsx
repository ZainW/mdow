import { createFileRoute, redirect } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getAllDocs } from '~/lib/content'

const fetchFirstSlug = createServerFn({ method: 'GET' }).handler(async () => {
  const docs = await getAllDocs()
  return docs[0]?.slug || 'getting-started'
})

export const Route = createFileRoute('/docs/')({
  beforeLoad: async () => {
    const slug = await fetchFirstSlug()
    throw redirect({ to: '/docs/$', params: { _splat: slug } })
  },
})
