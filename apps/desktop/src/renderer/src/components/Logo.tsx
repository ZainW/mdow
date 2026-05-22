import logoUrl from '../assets/mdow-logo.svg'
import { cn } from '../lib/utils'

type LogoProps = {
  className?: string
  alt?: string
}

export function Logo({ className, alt = 'Mdow' }: LogoProps) {
  return (
    <img
      src={logoUrl}
      alt={alt}
      width={48}
      height={48}
      className={cn('h-12 w-12 select-none', className)}
      draggable={false}
    />
  )
}
