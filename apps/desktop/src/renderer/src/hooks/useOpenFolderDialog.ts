import { useCallback } from 'react'
import { useAppStore } from '../store/app-store'

export function useOpenFolderDialog() {
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)

  return useCallback(async () => {
    const result = await window.api.openFolderDialog()
    if (result) {
      setOpenFolder(result.path, result.tree, result.truncated)
    }
  }, [setOpenFolder])
}
