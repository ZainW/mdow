import type { CSSProperties, ComponentType } from 'react'
import { cn } from '../lib/utils'
import { rovingTabIndex, useRovingFocus } from '../hooks/useRovingFocus'

export interface SegmentedOption<TValue extends string> {
  value: TValue
  label: string
  Icon?: ComponentType<{ className?: string; 'aria-hidden'?: boolean }>
  /** Extra inline style for the segment, e.g. a font preview. */
  style?: CSSProperties
}

/**
 * One recessed well with equal-width segments and a raised selected segment.
 * Exposed as a radiogroup with roving focus: Tab lands on the selected
 * segment, arrow keys move between segments, Enter/Space/click selects.
 */
export function SegmentedControl<TValue extends string>({
  label,
  value,
  options,
  onChange,
  className,
  segmentClassName,
}: {
  label: string
  value: TValue
  options: readonly SegmentedOption<TValue>[]
  onChange: (value: TValue) => void
  className?: string
  segmentClassName?: string
}) {
  const { containerRef, onKeyDown } = useRovingFocus({ orientation: 'horizontal' })

  return (
    // oxlint-disable-next-line jsx-a11y/interactive-supports-focus -- per WAI-ARIA, focus rests on the checked radio inside, not the radiogroup itself
    <div
      ref={containerRef}
      role="radiogroup"
      aria-label={label}
      onKeyDown={onKeyDown}
      className={cn(
        'segmented grid h-7 auto-cols-fr grid-flow-col gap-0.5 rounded-[7px] bg-surface-well p-0.5',
        className,
      )}
    >
      {options.map((opt) => {
        const checked = opt.value === value
        return (
          <button
            key={opt.value}
            type="button"
            // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- styled segments; native radio inputs can't take this layout
            role="radio"
            aria-checked={checked}
            aria-label={opt.label}
            title={opt.label}
            tabIndex={rovingTabIndex(checked)}
            data-checked={checked}
            onClick={() => onChange(opt.value)}
            className={cn(
              'segmented-item flex min-w-0 items-center justify-center gap-1.5 rounded-[5px] px-1.5 text-[length:var(--control-font-size)] font-medium outline-none',
              'focus-visible:ring-2 focus-visible:ring-ring/50',
              checked
                ? 'bg-surface-raised text-foreground shadow-(--shadow-raised) ring-[0.5px] ring-border'
                : 'text-muted-foreground hover:text-foreground',
              segmentClassName,
            )}
            style={opt.style}
          >
            {opt.Icon ? <opt.Icon className="size-3.5 shrink-0" aria-hidden /> : null}
            <span className="truncate">{opt.label}</span>
          </button>
        )
      })}
    </div>
  )
}
