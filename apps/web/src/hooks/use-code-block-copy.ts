import { useEffect, useState, type RefObject } from 'react'

export interface CodeBlockTarget {
  host: HTMLDivElement
  code: string
}

/**
 * Finds <pre> elements within a container and mounts portal host nodes
 * inside each one for rendering copy buttons. Re-runs when contentKey changes.
 */
export function useCodeBlockCopy(
  containerRef: RefObject<HTMLElement | null>,
  contentKey: string,
): CodeBlockTarget[] {
  const [targets, setTargets] = useState<CodeBlockTarget[]>([])

  useEffect(() => {
    const container = containerRef.current
    if (!container) {
      setTargets([])
      return
    }
    const pres = container.querySelectorAll<HTMLPreElement>('pre')
    const next: CodeBlockTarget[] = []
    for (const pre of pres) {
      pre.style.position = 'relative'
      const host = document.createElement('div')
      host.className = 'absolute right-2 top-2'
      pre.appendChild(host)
      next.push({ host, code: pre.textContent ?? '' })
    }
    setTargets(next)
    return () => {
      for (const { host } of next) host.remove()
    }
  }, [containerRef, contentKey])

  return targets
}
