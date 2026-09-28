import { useCallback, useRef, useState } from 'react'
import { summarizeDragItems, type DropSummary } from '../lib/drop-summary'

function isFileDrag(e: React.DragEvent): boolean {
  return Array.from(e.dataTransfer?.types ?? []).includes('Files')
}

/**
 * Tracks OS file drags over the window so a drop overlay can describe what is about to open.
 * Tab reordering and other in-app drags (no `Files` type) are ignored.
 */
export function useWindowFileDrop(onDrop: (e: React.DragEvent) => void) {
  const [summary, setSummary] = useState<DropSummary | null>(null)
  // dragenter/dragleave fire for every child crossed; only the outermost pair matters.
  const depth = useRef(0)

  const onDragEnter = useCallback((e: React.DragEvent) => {
    if (!isFileDrag(e)) return
    depth.current += 1
    if (depth.current === 1) setSummary(summarizeDragItems(Array.from(e.dataTransfer.items)))
  }, [])

  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
  }, [])

  const onDragLeave = useCallback((e: React.DragEvent) => {
    if (!isFileDrag(e)) return
    depth.current = Math.max(0, depth.current - 1)
    if (depth.current === 0) setSummary(null)
  }, [])

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      depth.current = 0
      setSummary(null)
      onDrop(e)
    },
    [onDrop],
  )

  return { summary, handlers: { onDragEnter, onDragOver, onDragLeave, onDrop: handleDrop } }
}
