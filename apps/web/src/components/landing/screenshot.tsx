import { cn } from '~/lib/utils'

export type ScreenshotName =
  | 'reading-light'
  | 'reading-dark'
  | 'empty-light'
  | 'empty-dark'
  | 'sidebar-light'
  | 'sidebar-dark'

interface ScreenshotProps {
  name: ScreenshotName
  alt: string
  className?: string
  /** Set on the hero image only — eager-loads, decodes sync, high fetch priority. */
  priority?: boolean
}

const WIDTH = 1999
const HEIGHT = 1361

export function Screenshot({ name, alt, className, priority = false }: ScreenshotProps) {
  return (
    <picture>
      <source srcSet={`/screenshots/${name}.avif`} type="image/avif" />
      <source srcSet={`/screenshots/${name}.webp`} type="image/webp" />
      <img
        src={`/screenshots/${name}.webp`}
        alt={alt}
        width={WIDTH}
        height={HEIGHT}
        loading={priority ? 'eager' : 'lazy'}
        decoding={priority ? 'sync' : 'async'}
        fetchPriority={priority ? 'high' : 'auto'}
        className={cn('h-auto w-full', className)}
      />
    </picture>
  )
}
