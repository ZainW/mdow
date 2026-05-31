import { useCallback, useEffect, useSyncExternalStore } from 'react'

type Theme = 'light' | 'dark'
type ThemePreference = 'light' | 'dark' | 'system'

const STORAGE_KEY = 'theme'
const TRANSITION_GUARD_MS = 50

function getSystemTheme(): Theme {
  if (typeof window === 'undefined') return 'light'
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function getStoredPreference(): ThemePreference | null {
  if (typeof window === 'undefined') return null
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === 'light' || stored === 'dark' || stored === 'system') return stored
  return null
}

function resolveTheme(preference: ThemePreference | null): Theme {
  if (preference === 'light' || preference === 'dark') return preference
  return getSystemTheme()
}

function applyTheme(theme: Theme) {
  const root = document.documentElement
  root.classList.add('theme-switching')
  root.classList.toggle('dark', theme === 'dark')
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      setTimeout(() => root.classList.remove('theme-switching'), TRANSITION_GUARD_MS)
    })
  })
}

function getSnapshot(): Theme {
  return document.documentElement.classList.contains('dark') ? 'dark' : 'light'
}

function getServerSnapshot(): Theme {
  return 'light'
}

function subscribe(callback: () => void): () => void {
  const media = window.matchMedia('(prefers-color-scheme: dark)')

  const onMediaChange = () => {
    const pref = getStoredPreference()
    if (pref === 'system' || pref === null) {
      const next = getSystemTheme()
      applyTheme(next)
      callback()
    }
  }

  const observer = new MutationObserver((mutations) => {
    for (const m of mutations) {
      if (m.attributeName === 'class') {
        callback()
        return
      }
    }
  })
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })

  media.addEventListener('change', onMediaChange)

  return () => {
    observer.disconnect()
    media.removeEventListener('change', onMediaChange)
  }
}

export function useTheme() {
  const theme = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot)

  useEffect(() => {
    const pref = getStoredPreference()
    const resolved = resolveTheme(pref)
    if (document.documentElement.classList.contains('dark') !== (resolved === 'dark')) {
      applyTheme(resolved)
    }
  }, [])

  const setTheme = useCallback((next: ThemePreference) => {
    localStorage.setItem(STORAGE_KEY, next)
    const resolved = resolveTheme(next)
    applyTheme(resolved)
  }, [])

  const toggleTheme = useCallback(() => {
    const current = document.documentElement.classList.contains('dark') ? 'dark' : 'light'
    const next: Theme = current === 'dark' ? 'light' : 'dark'
    localStorage.setItem(STORAGE_KEY, next)
    applyTheme(next)
  }, [])

  return { theme, setTheme, toggleTheme } as const
}
