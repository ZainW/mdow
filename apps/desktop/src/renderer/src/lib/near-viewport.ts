type Callback = () => void

interface SharedObserver {
  observer: IntersectionObserver
  callbacks: Map<Element, Callback>
}

// rootMargin only expands the observer's root, so lookahead needs the scroller itself as root —
// against the implicit viewport root the scroller's overflow clip would cancel the margin.
const SCROLLER_SELECTOR = '[data-markdown-scroller]'
const LOOKAHEAD = '100% 0px'

const observers = new Map<Element | null, SharedObserver>()

function getSharedObserver(root: Element | null): SharedObserver {
  let shared = observers.get(root)
  if (shared) return shared

  const callbacks = new Map<Element, Callback>()
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue
        const callback = callbacks.get(entry.target)
        callbacks.delete(entry.target)
        observer.unobserve(entry.target)
        callback?.()
      }
      if (callbacks.size === 0) {
        observer.disconnect()
        observers.delete(root)
      }
    },
    { root, rootMargin: LOOKAHEAD },
  )
  shared = { observer, callbacks }
  observers.set(root, shared)
  return shared
}

/**
 * Calls `callback` once, the first time `element` comes within a screen of the markdown
 * scroller's viewport. One IntersectionObserver is shared per scroller, so documents with
 * thousands of blocks do not pay for thousands of observers. Returns a disposer.
 */
export function onceNearViewport(element: Element, callback: Callback): () => void {
  if (typeof IntersectionObserver === 'undefined') {
    callback()
    return () => {}
  }

  const root = element.closest(SCROLLER_SELECTOR)
  const shared = getSharedObserver(root)
  shared.callbacks.set(element, callback)
  shared.observer.observe(element)

  return () => {
    if (!shared.callbacks.delete(element)) return
    shared.observer.unobserve(element)
    if (shared.callbacks.size === 0) {
      shared.observer.disconnect()
      observers.delete(root)
    }
  }
}
