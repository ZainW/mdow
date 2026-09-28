import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function isMacPlatform(): boolean {
  return window.api?.platform === 'darwin'
}

export const isMac = typeof window !== 'undefined' && window.api?.platform === 'darwin'

/**
 * Platform-appropriate label for a Cmd/Ctrl shortcut, e.g. `⇧⌘O` on macOS and
 * `Ctrl+Shift+O` elsewhere.
 */
export function formatShortcut(
  key: string,
  { shift = false, mac = isMac }: { shift?: boolean; mac?: boolean } = {},
): string {
  if (mac) return `${shift ? '⇧' : ''}⌘${key}`
  return ['Ctrl', shift ? 'Shift' : null, key].filter(Boolean).join('+')
}
