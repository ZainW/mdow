export function seo({
  title,
  description,
  image,
}: {
  title: string
  description: string
  image?: string
}) {
  const tags = [
    { title },
    { name: 'description', content: description },
    { name: 'og:title', content: title },
    { name: 'og:description', content: description },
    { name: 'twitter:card', content: 'summary_large_image' },
    { name: 'twitter:title', content: title },
    { name: 'twitter:description', content: description },
  ]

  if (image) {
    tags.push({ name: 'og:image', content: image })
    tags.push({ name: 'twitter:image', content: image })
  }

  return tags
}
