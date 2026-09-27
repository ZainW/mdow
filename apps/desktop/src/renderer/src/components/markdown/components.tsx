import { MarkdownDocument } from '@comark/react'
import {
  lazy,
  memo,
  Suspense,
  useMemo,
  type AnchorHTMLAttributes,
  type HTMLAttributes,
  type ImgHTMLAttributes,
} from 'react'

const ComarkMath = lazy(() =>
  import('@comark/react/components/Math').then((mod) => ({ default: mod.Math })),
)

import type { RenderResult } from '../../lib/markdown'
import { resolveRelativePath } from '../../lib/path-utils'
import { SECTION_TAG } from '../../lib/markdown-sections'
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

interface MarkdownSectionProps {
  estimate?: number
  signature?: string
  children?: React.ReactNode
}

// Sections whose content signature is unchanged skip re-rendering entirely, so a live reload of
// a long document only touches what changed.
const MarkdownSection = memo(
  function MarkdownSection({ estimate, children }: MarkdownSectionProps): React.JSX.Element {
    return (
      <div
        className="md-section"
        style={
          typeof estimate === 'number'
            ? ({ '--md-section-estimate': `${estimate}em` } as React.CSSProperties)
            : undefined
        }
      >
        {children}
      </div>
    )
  },
  (prev, next) => prev.signature !== undefined && prev.signature === next.signature,
)

function createMarkdownComponents(docPath: string) {
  const alertComponents = Object.fromEntries(
    ALERT_TYPES.map((type) => [
      type,
      (props: HTMLAttributes<HTMLDivElement>) => <AlertCallout type={type} {...props} />,
    ]),
  )

  return {
    [SECTION_TAG]: MarkdownSection,
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
  return (
    <Suspense fallback={null}>
      <MarkdownDocument value={result.tree} components={components} />
    </Suspense>
  )
})
