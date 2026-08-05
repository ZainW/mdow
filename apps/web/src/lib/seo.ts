export const SITE_URL = 'https://mdow.wania.app'

export function absoluteUrl(path: string) {
  return new URL(path, SITE_URL).toString()
}

export function canonical(path: string) {
  return { rel: 'canonical', href: absoluteUrl(path) }
}

export function jsonLd(data: Record<string, unknown>) {
  return JSON.stringify(data).replace(/</g, '\\u003C')
}

export function seo({
  title,
  description,
  image,
  path = '/',
}: {
  title: string
  description: string
  image?: string
  path?: string
}) {
  const imageUrl = absoluteUrl(image || '/og-image.png')

  const tags = [
    { title },
    { name: 'description', content: description },
    { property: 'og:type', content: 'website' },
    { property: 'og:site_name', content: 'Mdow' },
    { property: 'og:title', content: title },
    { property: 'og:description', content: description },
    { property: 'og:url', content: absoluteUrl(path) },
    { property: 'og:image', content: imageUrl },
    { name: 'twitter:card', content: 'summary_large_image' },
    { name: 'twitter:title', content: title },
    { name: 'twitter:description', content: description },
    { name: 'twitter:image', content: imageUrl },
  ]

  return tags
}
