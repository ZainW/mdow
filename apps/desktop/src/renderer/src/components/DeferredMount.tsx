import { Suspense, useState, type ReactNode } from 'react'

/**
 * Renders nothing until `when` first becomes true, then stays mounted so exit animations and
 * internal state survive closing. Pair with `lazy()` to keep rarely used UI out of startup.
 */
export function DeferredMount({ when, children }: { when: boolean; children: ReactNode }) {
  const [mounted, setMounted] = useState(when)
  if (when && !mounted) setMounted(true)
  if (!mounted) return null
  return <Suspense fallback={null}>{children}</Suspense>
}
