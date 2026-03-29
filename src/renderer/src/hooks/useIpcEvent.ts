import { useEffect } from 'react'

export function useIpcEvent<T>(
  subscribe: (callback: (data: T) => void) => () => void,
  handler: (data: T) => void,
): void {
  useEffect(() => {
    const unsubscribe = subscribe(handler)
    return unsubscribe
  }, [subscribe, handler])
}
