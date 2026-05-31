import { useCallback, useEffect } from 'react'
import type { ScanResult } from '../../../shared/types'
import { useIpcEvent } from './useIpcEvent'
import { useAppStore } from '../store/app-store'

export function useFolderTree(folderPath: string | null) {
  const setFolderTree = useAppStore((s) => s.setFolderTree)

  useEffect(() => {
    if (!folderPath) return
    let cancelled = false
    void window.api.readFolderTree(folderPath).then((scan) => {
      if (!cancelled) setFolderTree(scan.tree, scan.truncated)
    })
    return () => {
      cancelled = true
    }
  }, [folderPath, setFolderTree])

  const handleFolderChanged = useCallback(
    (scan: ScanResult) => {
      if (folderPath) setFolderTree(scan.tree, scan.truncated)
    },
    [folderPath, setFolderTree],
  )

  useIpcEvent(window.api.onFolderChanged, handleFolderChanged)
}
