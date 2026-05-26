import { useEffect, useState } from 'react'

const DISMISS_KEY = 'mdow-download-bar-dismissed'

/** True when the user has scrolled past the hero threshold. */
export function useScrollPast(thresholdPx: number): boolean {
  const [past, setPast] = useState(false)

  useEffect(() => {
    function onScroll() {
      setPast(window.scrollY > thresholdPx)
    }
    onScroll()
    window.addEventListener('scroll', onScroll, { passive: true })
    return () => window.removeEventListener('scroll', onScroll)
  }, [thresholdPx])

  return past
}

export function useDownloadBarDismissed(): [boolean, () => void] {
  const [dismissed, setDismissed] = useState(true)

  useEffect(() => {
    setDismissed(sessionStorage.getItem(DISMISS_KEY) === '1')
  }, [])

  const dismiss = () => {
    sessionStorage.setItem(DISMISS_KEY, '1')
    setDismissed(true)
  }

  return [dismissed, dismiss]
}
