import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'
import { useIntroSkip } from '~/hooks/use-intro-seen'

interface IntroSectionProps {
  children: ReactNode
  className?: string
  delay?: 0 | 1 | 2
}

export function IntroSection({ children, className, delay = 0 }: IntroSectionProps) {
  const skip = useIntroSkip()

  return (
    <div
      className={cn(
        !skip && 'intro-animate',
        !skip && delay === 1 && 'intro-animate-delay-1',
        !skip && delay === 2 && 'intro-animate-delay-2',
        className,
      )}
    >
      {children}
    </div>
  )
}
