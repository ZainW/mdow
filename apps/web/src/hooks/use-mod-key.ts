import { useEffect, useState } from 'react'

/** Returns ⌘ on Mac, Ctrl elsewhere. Empty string until mounted (SSR-safe). */
export function useModKey(): string {
  const [mod, setMod] = useState('')

  useEffect(() => {
    const isMac = /Mac|iPhone|iPad|iPod/.test(navigator.platform)
    setMod(isMac ? '⌘' : 'Ctrl')
  }, [])

  return mod
}

/** Format a keyboard shortcut label, e.g. modKey="⌘" → "⌘K" */
export function formatShortcut(modKey: string, key: string): string {
  if (!modKey) return key
  return `${modKey}${key}`
}
