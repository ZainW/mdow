import { useCallback, useEffect } from 'react'
import { useAppStore } from '../store/app-store'

function applyDarkClass(isDark: boolean): void {
  document.documentElement.classList.toggle('dark', isDark)
}

export function useTheme(): void {
  const theme = useAppStore((s) => s.theme)

  useEffect(() => {
    if (theme === 'light') {
      applyDarkClass(false)
      return
    }
    if (theme === 'dark') {
      applyDarkClass(true)
      return
    }

    // System: matchMedia is a reasonable initial value until onThemeChanged fires.
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    applyDarkClass(mq.matches)
  }, [theme])

  const handleThemeChanged = useCallback((isDark: boolean) => {
    if (useAppStore.getState().theme === 'system') {
      applyDarkClass(isDark)
    }
  }, [])

  useEffect(() => {
    const unsubscribe = window.api.onThemeChanged(handleThemeChanged)
    return unsubscribe
  }, [handleThemeChanged])
}
