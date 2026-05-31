import { ComarkRenderer } from '@comark/react'
import { Math as ComarkMath } from '@comark/react/components/Math'
import {
  memo,
  useMemo,
  type AnchorHTMLAttributes,
  type HTMLAttributes,
  type ImgHTMLAttributes,
} from 'react'
import type { RenderResult } from '../../lib/markdown'
import { resolveRelativePath } from '../../lib/path-utils'
import { ALERT_TYPES, AlertCallout } from './AlertCallout'
import { CodeBlock } from './CodeBlock'
import { MermaidBlock } from './MermaidBlock'
import { TableWrap } from './TableWrap'
import { TaskCheckbox } from './TaskCheckbox'

function rewriteImageSrc(src: string, docPath: string): string {
  if (/^(https?:|data:|mdow-local:|blob:)/i.test(src)) return src
  const resolved = resolveRelativePath(docPath, src)
  return `mdow-local://local/${encodeURIComponent(resolved)}`
}

function createMarkdownComponents(docPath: string) {
  const alertComponents = Object.fromEntries(
    ALERT_TYPES.map((type) => [
      type,
      (props: HTMLAttributes<HTMLDivElement>) => <AlertCallout type={type} {...props} />,
    ]),
  )

  return {
    pre: CodeBlock,
    mermaid: MermaidBlock,
    math: ComarkMath,
    table: TableWrap,
    input: TaskCheckbox,
    img: ({ src, alt, ...props }: ImgHTMLAttributes<HTMLImageElement>) => (
      <img
        src={src ? rewriteImageSrc(src, docPath) : src}
        alt={alt ?? ''}
        loading="lazy"
        {...props}
      />
    ),
    a: ({ children, ...props }: AnchorHTMLAttributes<HTMLAnchorElement>) => (
      <a {...props}>{children ?? props.href}</a>
    ),
    ...alertComponents,
  }
}

export const MarkdownContent = memo(function MarkdownContent({
  result,
  docPath,
}: {
  result: RenderResult
  docPath: string
}) {
  const components = useMemo(() => createMarkdownComponents(docPath), [docPath])
  return <ComarkRenderer tree={result.tree} components={components} />
})
