import { useCallback, useEffect } from 'react'

export function useTheme(): void {
  const handleThemeChanged = useCallback((isDark: boolean) => {
    document.documentElement.classList.toggle('dark', isDark)
  }, [])

  useEffect(() => {
    const unsubscribe = window.api.onThemeChanged(handleThemeChanged)
    return unsubscribe
  }, [handleThemeChanged])
}
