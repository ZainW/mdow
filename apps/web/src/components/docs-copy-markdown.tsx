import { CopyButton } from '~/components/copy-button'

interface DocsCopyMarkdownProps {
  markdown: string
  slug: string
}

export function DocsCopyMarkdown({ markdown, slug }: DocsCopyMarkdownProps) {
  return (
    <div className="not-prose mb-8 flex flex-wrap items-center gap-3 rounded-lg border border-border-subtle bg-surface px-4 py-3 text-sm">
      <span className="text-muted-foreground">Copy this page as Markdown</span>
      <CopyButton value={markdown} />
      <a
        href={`/docs/${slug}.md`}
        className="text-primary underline underline-offset-2 hover:text-primary/80"
      >
        Raw .md
      </a>
    </div>
  )
}
