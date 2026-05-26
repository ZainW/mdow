import { useEffect, useState } from 'react'

const STORAGE_KEY = 'mdow-intro-seen'

/** True when intro animation should be skipped (already seen this session). */
export function useIntroSkip(): boolean {
  const [skip, setSkip] = useState(true)

  useEffect(() => {
    const already = sessionStorage.getItem(STORAGE_KEY)
    if (already) {
      setSkip(true)
      return
    }
    sessionStorage.setItem(STORAGE_KEY, '1')
    setSkip(false)
  }, [])

  return skip
}
