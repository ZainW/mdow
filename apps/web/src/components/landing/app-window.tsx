import { cn } from '~/lib/utils'
import { Screenshot, type ScreenshotName } from './screenshot'

interface AppWindowProps {
  name: ScreenshotName
  alt: string
  priority?: boolean
  className?: string
}

/**
 * A screenshot of the app presented as a macOS window. Captures are web
 * content only, so the traffic lights are drawn here, positioned in
 * percentages so they scale with the image.
 */
export function AppWindow({ name, alt, priority, className }: AppWindowProps) {
  return (
    <div className={cn('window-frame relative overflow-hidden', className)}>
      <Screenshot name={name} alt={alt} priority={priority} />
      <div
        aria-hidden
        className="absolute left-[1.3%] top-[2.1%] flex gap-[0.45%] [&>span]:aspect-square [&>span]:w-[0.75%] [&>span]:rounded-full"
        style={{ width: '100%' }}
      >
        <span className="bg-[#ff5f57] shadow-[inset_0_0_0_0.5px_oklch(0_0_0/0.12)]" />
        <span className="bg-[#febc2e] shadow-[inset_0_0_0_0.5px_oklch(0_0_0/0.12)]" />
        <span className="bg-[#28c840] shadow-[inset_0_0_0_0.5px_oklch(0_0_0/0.12)]" />
      </div>
    </div>
  )
}
