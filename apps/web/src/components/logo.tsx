import { cn } from '~/lib/utils'

type LogoProps = {
  className?: string
  alt?: string
}

export function Logo({ className, alt = 'Mdow' }: LogoProps) {
  return (
    <img
      src="/mdow-logo.svg"
      alt={alt}
      width={28}
      height={28}
      className={cn('h-7 w-7 select-none', className)}
      draggable={false}
    />
  )
}
