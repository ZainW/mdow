import { scrollToTarget } from './scroll-to'

/**
 * A scroll position expressed as "this block, this many pixels above the viewport top". Pixel
 * offsets drift in long documents, where blocks far from the viewport only have estimated
 * heights; anchoring to a block lands on the same content every time.
 */
export interface ScrollAnchor {
  /** Child indexes from `.comark-content` down to the anchor block (sections add a level). */
  path: number[]
  /** How far the viewport top sits below the anchor block's top, in pixels. */
  offset: number
}

function firstBelow(elements: HTMLCollection, line: number): number {
  let low = 0
  let high = elements.length - 1
  let found = -1
  while (low <= high) {
    const mid = (low + high) >> 1
    if (elements[mid].getBoundingClientRect().bottom > line) {
      found = mid
      high = mid - 1
    } else {
      low = mid + 1
    }
  }
  return found
}

function contentRoot(container: HTMLElement): HTMLElement | null {
  return container.querySelector<HTMLElement>('.comark-content')
}

export function captureScrollAnchor(
  scroller: HTMLElement,
  container: HTMLElement,
): ScrollAnchor | null {
  const root = contentRoot(container)
  if (!root || scroller.scrollTop === 0) return null

  const line = scroller.getBoundingClientRect().top
  const path: number[] = []
  let parent: Element = root
  let block: Element | null = null
  // Descend at most two levels: top-level block, then (for sections) the block inside it.
  for (let depth = 0; depth < 2; depth++) {
    const index = firstBelow(parent.children, line)
    if (index < 0) break
    path.push(index)
    block = parent.children[index]
    if (!block.classList.contains('md-section')) break
    parent = block
  }
  if (!block) return null
  return { path, offset: line - block.getBoundingClientRect().top }
}

export function resolveScrollAnchor(container: HTMLElement, anchor: ScrollAnchor): Element | null {
  let element: Element | null = contentRoot(container)
  for (const index of anchor.path) {
    element = element?.children[index] ?? null
  }
  return element
}

/** Restores an anchor, falling back to a raw pixel position. Returns a cancel function. */
export function restoreScrollPosition(
  scroller: HTMLElement,
  container: HTMLElement,
  anchor: ScrollAnchor | null | undefined,
  fallbackTop: number,
): () => void {
  const target = anchor ? resolveScrollAnchor(container, anchor) : null
  if (!anchor || !target) {
    scroller.scrollTop = fallbackTop
    return () => {}
  }
  return scrollToTarget(scroller, target, { offset: -anchor.offset })
}
