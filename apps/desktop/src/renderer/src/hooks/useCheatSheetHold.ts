import { useEffect, useRef, useState } from 'react'
import { useAppStore } from '../store/app-store'
import { CHEAT_SHEET_HOLD_MS, isCheatSheetHoldKey } from '../lib/cheat-sheet'

function modalOwnsScreen(): boolean {
  const state = useAppStore.getState()
  return (
    state.commandPaletteOpen || state.shortcutsDialogOpen || state.settingsOpen || state.searchOpen
  )
}

export function useCheatSheetHold(): boolean {
  const [open, setOpen] = useState(false)
  const openRef = useRef(false)
  const timerRef = useRef<number | undefined>(undefined)
  const suppressUntilReleaseRef = useRef(false)

  openRef.current = open

  useEffect(() => {
    const clearTimer = () => {
      if (timerRef.current === undefined) return
      window.clearTimeout(timerRef.current)
      timerRef.current = undefined
    }

    const hide = () => {
      clearTimer()
      if (openRef.current) setOpen(false)
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (isCheatSheetHoldKey(event)) {
        if (modalOwnsScreen() || suppressUntilReleaseRef.current) return
        if (timerRef.current !== undefined || openRef.current) return
        timerRef.current = window.setTimeout(() => {
          timerRef.current = undefined
          if (modalOwnsScreen() || suppressUntilReleaseRef.current) return
          setOpen(true)
        }, CHEAT_SHEET_HOLD_MS)
        return
      }

      clearTimer()
      if (!openRef.current) return
      if (event.key === 'Escape') {
        event.preventDefault()
        suppressUntilReleaseRef.current = true
      }
      setOpen(false)
    }

    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key !== 'Meta' && event.key !== 'Control') return
      suppressUntilReleaseRef.current = false
      hide()
    }

    const onBlur = () => {
      suppressUntilReleaseRef.current = false
      hide()
    }

    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keyup', onKeyUp)
    window.addEventListener('blur', onBlur)
    const unsubscribe = useAppStore.subscribe((state) => {
      if (
        state.commandPaletteOpen ||
        state.shortcutsDialogOpen ||
        state.settingsOpen ||
        state.searchOpen
      ) {
        hide()
      }
    })

    return () => {
      clearTimer()
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('keyup', onKeyUp)
      window.removeEventListener('blur', onBlur)
      unsubscribe()
    }
  }, [])

  return open
}
