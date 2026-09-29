import { cn } from '~/lib/utils'

export type ScreenshotName = 'reading' | 'outline' | 'palette' | 'empty'

export const SCREENSHOT_SIZE = { width: 2000, height: 1250 }

interface ScreenshotProps {
  /** Base name; `-light` / `-dark` variants live in /public/screenshots. */
  name: ScreenshotName
  alt: string
  className?: string
  /** Above-the-fold image: raise fetch priority. */
  priority?: boolean
}

/**
 * Renders the light and dark captures of the app and lets the site theme pick
 * one via CSS. Both images are lazy so the hidden (display: none) variant is
 * never fetched.
 */
export function Screenshot({ name, alt, className, priority = false }: ScreenshotProps) {
  return (
    <div className={cn('overflow-hidden', className)}>
      <Variant name={`${name}-light`} alt={alt} priority={priority} className="block dark:hidden" />
      <Variant name={`${name}-dark`} alt={alt} priority={priority} className="hidden dark:block" />
    </div>
  )
}

function Variant({
  name,
  alt,
  priority,
  className,
}: {
  name: string
  alt: string
  priority: boolean
  className: string
}) {
  return (
    <picture className={className}>
      <source srcSet={`/screenshots/${name}.avif`} type="image/avif" />
      <source srcSet={`/screenshots/${name}.webp`} type="image/webp" />
      <img
        src={`/screenshots/${name}.webp`}
        alt={alt}
        width={SCREENSHOT_SIZE.width}
        height={SCREENSHOT_SIZE.height}
        loading="lazy"
        decoding="async"
        fetchPriority={priority ? 'high' : 'auto'}
        className="block h-auto w-full"
      />
    </picture>
  )
}
