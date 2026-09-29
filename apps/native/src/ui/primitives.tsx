import type { ReactNode } from 'react'
import type { EventPayload, StyleDesc } from '@gpuix/react'
import { withAlpha } from '../lib/theme'
import { ICONS, type IconName } from './icons'
import { UI_FONT, useUi } from './context'

/** `filled` paints the shape solid, as the desktop does for an active choice's icon. */
export function Icon({
  name,
  size = 16,
  color,
  filled,
}: {
  name: IconName
  size?: number
  color?: string
  filled?: boolean
}) {
  const { theme } = useUi()
  return (
    <svg
      source={filled ? ICONS[name].replace('fill="none"', 'fill="currentColor"') : ICONS[name]}
      style={{ width: size, height: size, flexShrink: 0, color: color ?? theme.mutedForeground }}
    />
  )
}

export function Label({
  children,
  size,
  color,
  weight,
  mono,
  style,
}: {
  children: ReactNode
  size?: number
  color?: string
  weight?: number
  mono?: boolean
  style?: StyleDesc
}) {
  const { theme, scale } = useUi()
  return (
    <text
      style={{
        fontFamily: mono ? 'Geist Mono' : UI_FONT,
        fontSize: size ?? scale.controlFont + 1,
        fontWeight: weight ?? 400,
        color: color ?? theme.foreground,
        ...style,
      }}
    >
      {children}
    </text>
  )
}

export function IconButton({
  icon,
  label,
  onClick,
  active,
  size,
  testId,
}: {
  icon: IconName
  label: string
  onClick: () => void
  active?: boolean
  size?: number
  testId?: string
}) {
  const { theme, scale } = useUi()
  const box = size ?? scale.buttonHeight
  return (
    <div
      role="button"
      aria-label={label}
      testId={testId}
      tabIndex={0}
      onClick={onClick}
      onKeyDown={activateOnEnter(onClick)}
      style={{
        width: box,
        height: box,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        borderRadius: 6,
        cursor: 'pointer',
        flexShrink: 0,
        backgroundColor: active ? theme.sidebarAccent : undefined,
        hover: { backgroundColor: theme.sidebarAccent },
        userSelect: 'none',
      }}
    >
      <Icon name={icon} color={active ? theme.foreground : theme.mutedForeground} />
    </div>
  )
}

export function Button({
  children,
  icon,
  onClick,
  variant = 'outline',
  testId,
  small,
  muted,
}: {
  children: ReactNode
  icon?: IconName
  onClick: () => void
  variant?: 'outline' | 'primary' | 'ghost' | 'destructive'
  testId?: string
  small?: boolean
  /** Muted label, as for buttons sitting in muted copy (the welcome screen). */
  muted?: boolean
}) {
  const { theme, scale } = useUi()
  const color =
    variant === 'primary'
      ? theme.background
      : variant === 'destructive'
        ? theme.destructive
        : muted
          ? theme.mutedForeground
          : theme.foreground
  // shadcn's outline button fills with `input/30` in dark mode; `border` is the same colour.
  const outlineFill = theme.scheme === 'dark' ? withAlpha(theme.border, 0.3) : undefined
  const outlineHover = theme.scheme === 'dark' ? withAlpha(theme.border, 0.5) : theme.muted
  return (
    <div
      role="button"
      testId={testId}
      tabIndex={0}
      onClick={onClick}
      onKeyDown={activateOnEnter(onClick)}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: small ? 4 : 6,
        height: small ? scale.buttonXsHeight + 4 : scale.buttonHeight + 4,
        paddingLeft: small ? (icon ? 6 : 8) : 12,
        paddingRight: small ? 8 : 12,
        borderRadius: 6,
        borderWidth: variant === 'outline' ? 1 : 0,
        borderColor: theme.border,
        backgroundColor:
          variant === 'primary'
            ? theme.foreground
            : variant === 'outline'
              ? outlineFill
              : undefined,
        cursor: 'pointer',
        flexShrink: 0,
        userSelect: 'none',
        hover: {
          backgroundColor:
            variant === 'primary'
              ? theme.mutedForeground
              : variant === 'outline'
                ? outlineHover
                : theme.sidebarAccent,
        },
      }}
    >
      {icon ? <Icon name={icon} color={color} size={small ? 12 : 16} /> : null}
      <Label color={color} size={small ? scale.controlFont : undefined} weight={500}>
        {children}
      </Label>
    </div>
  )
}

/** Desktop `<Kbd>`: a borderless 20px muted chip with 10px medium text. */
export function Kbd({ children }: { children: string }) {
  const { theme } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: 20,
        minWidth: 20,
        paddingLeft: 4,
        paddingRight: 4,
        borderRadius: 4,
        backgroundColor: theme.muted,
        flexShrink: 0,
      }}
    >
      <Label size={10} weight={500} color={theme.mutedForeground}>
        {children}
      </Label>
    </div>
  )
}

export function Segmented<T extends string>({
  value,
  options,
  labels,
  onChange,
}: {
  value: T
  options: readonly T[]
  labels: Record<T, string>
  onChange: (value: T) => void
}) {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        padding: 2,
        gap: 2,
        borderRadius: 8,
        backgroundColor: theme.muted,
        flexShrink: 0,
      }}
    >
      {options.map((option) => {
        const selected = option === value
        return (
          <div
            key={option}
            role="tab"
            aria-selected={selected}
            tabIndex={0}
            onClick={() => onChange(option)}
            onKeyDown={activateOnEnter(() => onChange(option))}
            style={{
              paddingLeft: 10,
              paddingRight: 10,
              height: scale.buttonXsHeight + 4,
              display: 'flex',
              alignItems: 'center',
              borderRadius: 6,
              cursor: 'pointer',
              backgroundColor: selected ? theme.surfaceRaised : undefined,
              boxShadow: selected
                ? { offsetX: 0, offsetY: 1, blurRadius: 2, spreadRadius: 0, color: '#0000001f' }
                : undefined,
              hover: selected ? undefined : { backgroundColor: theme.sidebarAccent },
              userSelect: 'none',
            }}
          >
            <Label
              size={scale.controlFont}
              weight={selected ? 500 : 400}
              color={selected ? theme.foreground : theme.mutedForeground}
            >
              {labels[option]}
            </Label>
          </div>
        )
      })}
    </div>
  )
}

export function activateOnEnter(action: () => void) {
  return (event: EventPayload) => {
    if (event.key === 'enter' || event.key === 'space') action()
  }
}

/** Tailwind-style `/75` alpha modifiers for theme colours. */
export { withAlpha }

export function EmptyState({
  icon,
  title,
  body,
  detail,
  size = 'md',
  children,
}: {
  icon: IconName
  title: string
  body: string
  detail?: string
  /** `sm` is the compact, top-aligned sidebar variant. */
  size?: 'sm' | 'md'
  children?: ReactNode
}) {
  const { theme, scale } = useUi()
  if (size === 'sm') {
    return (
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 6,
          paddingLeft: 12,
          paddingRight: 12,
          paddingTop: 24,
          paddingBottom: 24,
        }}
      >
        <Icon name={icon} size={20} color={withAlpha(theme.mutedForeground, 0.4)} />
        <Label
          size={scale.controlFont}
          color={withAlpha(theme.foreground, 0.8)}
          style={{ textAlign: 'center', lineHeight: 16 }}
        >
          {title}
        </Label>
        <Label
          size={scale.controlFont - 1}
          color={withAlpha(theme.mutedForeground, 0.9)}
          style={{ textAlign: 'center', lineHeight: 15, maxWidth: 158 }}
        >
          {body}
        </Label>
        {detail ? (
          <Label
            mono
            size={scale.controlFont - 1}
            color={theme.mutedForeground}
            style={{ textAlign: 'center', maxWidth: 200 }}
          >
            {detail}
          </Label>
        ) : null}
        {children ? <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>{children}</div> : null}
      </div>
    )
  }
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 8,
        padding: 24,
        maxWidth: 440,
      }}
    >
      <div
        style={{
          width: 44,
          height: 44,
          borderRadius: 22,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          backgroundColor: theme.muted,
          marginBottom: 8,
        }}
      >
        <Icon name={icon} size={20} />
      </div>
      <Label size={scale.controlFont + 5} weight={500} style={{ textAlign: 'center' }}>
        {title}
      </Label>
      <Label
        size={scale.controlFont + 3}
        color={theme.mutedForeground}
        style={{ textAlign: 'center' }}
      >
        {body}
      </Label>
      {detail ? (
        <Label
          mono
          size={scale.controlFont}
          color={theme.mutedForeground}
          style={{ textAlign: 'center' }}
        >
          {detail}
        </Label>
      ) : null}
      {children ? <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>{children}</div> : null}
    </div>
  )
}
