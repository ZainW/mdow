import { isMac } from '@renderer/lib/utils'

export function TitlebarInset(): React.JSX.Element | null {
  if (!isMac) return null
  return <div aria-hidden className="titlebar-drag shrink-0" />
}
