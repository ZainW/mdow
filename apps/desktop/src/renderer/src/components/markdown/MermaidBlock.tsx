import { useEffect, useRef, useState, type HTMLAttributes } from 'react'
import { renderMermaidBlock } from '../../lib/mermaid'
import { cn } from '../../lib/utils'

export interface MermaidBlockProps extends HTMLAttributes<HTMLDivElement> {
  content?: string
}

export function MermaidBlock({ content, id, className, ...props }: MermaidBlockProps) {
  const ref = useRef<HTMLDivElement>(null)
  const [ariaLabel, setAriaLabel] = useState('Mermaid diagram loading')

  useEffect(() => {
    const el = ref.current
    if (!el || !content) return undefined

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry?.isIntersecting) return
        observer.disconnect()
        const blockId = el.id
        if (!blockId) return
        void renderMermaidBlock({ id: blockId, code: content }).then(() => {
          setAriaLabel('Mermaid diagram')
        })
      },
      { rootMargin: '200px 0px' },
    )

    observer.observe(el)
    return () => observer.disconnect()
  }, [content, id])

  return (
    <div
      ref={ref}
      id={id}
      className={cn('mermaid mermaid-container', className)}
      aria-label={ariaLabel}
      {...props}
    />
  )
}
