import { useCallback, useEffect } from 'react'

export function useTheme(): void {
  const handleThemeChanged = useCallback((isDark: boolean) => {
    document.documentElement.classList.toggle('dark', isDark)
  }, [])

  useEffect(() => {
    // Set initial theme from OS preference (reflects nativeTheme in Electron)
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    document.documentElement.classList.toggle('dark', mq.matches)

    const unsubscribe = window.api.onThemeChanged(handleThemeChanged)
    return unsubscribe
  }, [handleThemeChanged])
}
