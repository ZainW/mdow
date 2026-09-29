import { CopyButton } from '~/components/copy-button'
import { FileIcon } from './icons'

interface DocsCopyMarkdownProps {
  markdown: string
  slug: string
}

/** Page actions: copy the page as Markdown, or open the raw .md file. */
export function DocsCopyMarkdown({ markdown, slug }: DocsCopyMarkdownProps) {
  return (
    <div className="flex items-center gap-1.5">
      <CopyButton value={markdown} label="Copy page" />
      <a
        href={`/docs/${slug}.md`}
        className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border-subtle bg-card px-2 text-xs font-medium text-muted-foreground transition-[background-color,color] duration-150 ease hover:bg-muted hover:text-foreground"
      >
        <FileIcon className="size-3.5" />
        .md
      </a>
    </div>
  )
}
