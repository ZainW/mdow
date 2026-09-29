import { cn } from '~/lib/utils'

type LogoProps = {
  className?: string
  alt?: string
}

/**
 * The app icon tile. The cream tile blends into the paper background, so it
 * gets a hairline ring and a small shadow to read as an icon.
 */
export function Logo({ className, alt = 'Mdow' }: LogoProps) {
  return (
    <img
      src="/mdow-logo.svg"
      alt={alt}
      width={28}
      height={28}
      className={cn(
        'h-7 w-7 select-none rounded-[22%] shadow-[0_0_0_1px_oklch(0_0_0/0.08),0_1px_2px_oklch(0_0_0/0.08)] dark:shadow-[0_0_0_1px_oklch(1_0_0/0.1)]',
        className,
      )}
      draggable={false}
    />
  )
}
