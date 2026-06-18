import { useEffect, useMemo, useState } from 'react'
import { useAppStore, type Tab } from '../store/app-store'
import { basename } from '../lib/path-utils'
import { DocumentSkeleton } from './DocumentSkeleton'
import { ZoomIndicator } from './ZoomIndicator'

function hashContent(content: string): string {
  let hash = 0
  for (let i = 0; i < content.length; i++) {
    hash = (hash << 5) - hash + content.charCodeAt(i)
    hash |= 0
  }
  return String(hash)
}

export function HtmlView({ tab }: { tab: Tab }) {
  const zoomLevel = useAppStore((s) => s.zoomLevel)
  const [localUrl, setLocalUrl] = useState('')
  const [failed, setFailed] = useState(false)
  const version = useMemo(() => hashContent(tab.content), [tab.content])

  useEffect(() => {
    let cancelled = false
    setFailed(false)
    void window.api.resolveLocalUrl(tab.path).then((url) => {
      if (cancelled) return
      if (!url) {
        setFailed(true)
        setLocalUrl('')
        return
      }
      setLocalUrl(url)
    })
    return () => {
      cancelled = true
    }
  }, [tab.path])

  const source = localUrl ? `${localUrl}?v=${encodeURIComponent(version)}` : ''
  const scale = zoomLevel / 100

  return (
    <div className="relative flex flex-1 flex-col overflow-hidden bg-background">
      {failed ? (
        <div className="mx-auto w-full max-w-2xl px-12 py-8">
          <div className="rounded-lg border border-destructive/20 bg-destructive/5 p-4 text-sm">
            <p className="text-destructive">This HTML document could not be rendered.</p>
          </div>
        </div>
      ) : source ? (
        <div className="min-h-0 flex-1 overflow-auto bg-white dark:bg-background">
          <iframe
            key={source}
            title={`${basename(tab.path)} preview`}
            src={source}
            sandbox=""
            className="block h-full min-h-full w-full border-0 bg-white"
            style={{
              zoom: scale,
              width: `${100 / scale}%`,
              height: `${100 / scale}%`,
            }}
          />
        </div>
      ) : (
        <div className="mx-auto w-full max-w-3xl px-12 py-8">
          <DocumentSkeleton />
        </div>
      )}
      <ZoomIndicator />
    </div>
  )
}
