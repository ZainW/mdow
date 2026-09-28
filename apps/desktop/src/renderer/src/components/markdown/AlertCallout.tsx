import type { HTMLAttributes } from 'react'
import {
  Info,
  Lightbulb,
  MessageSquareWarning,
  OctagonAlert,
  TriangleAlert,
  type LucideIcon,
} from 'lucide-react'
import { cn } from '../../lib/utils'

export const ALERT_TYPES = ['tip', 'note', 'important', 'warning', 'caution'] as const
type AlertType = (typeof ALERT_TYPES)[number]

const ALERT_META: Record<AlertType, { title: string; Icon: LucideIcon }> = {
  note: { title: 'Note', Icon: Info },
  tip: { title: 'Tip', Icon: Lightbulb },
  important: { title: 'Important', Icon: MessageSquareWarning },
  warning: { title: 'Warning', Icon: TriangleAlert },
  caution: { title: 'Caution', Icon: OctagonAlert },
}

function isAlertType(type: string): type is AlertType {
  return type in ALERT_META
}

export function AlertCallout({
  type,
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement> & { type: string }) {
  const meta = isAlertType(type) ? ALERT_META[type] : null
  return (
    <div
      className={cn('markdown-alert', `markdown-alert-${type}`, className)}
      role="note"
      aria-label={meta?.title}
      {...props}
    >
      {meta && (
        <p className="markdown-alert-title" aria-hidden>
          <meta.Icon className="markdown-alert-icon" strokeWidth={2} />
          {meta.title}
        </p>
      )}
      <div className="markdown-alert-body">{children}</div>
    </div>
  )
}
