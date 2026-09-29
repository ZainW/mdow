import type { ReactNode } from 'react'
import type { EventPayload, StyleDesc } from '@gpuix/react'
import { ICONS, type IconName } from './icons'
import { UI_FONT, useUi } from './context'

export function Icon({
  name,
  size = 16,
  color,
}: {
  name: IconName
  size?: number
  color?: string
}) {
  const { theme } = useUi()
  return (
    <svg
      source={ICONS[name]}
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
}: {
  children: ReactNode
  icon?: IconName
  onClick: () => void
  variant?: 'outline' | 'primary' | 'ghost' | 'destructive'
  testId?: string
  small?: boolean
}) {
  const { theme, scale } = useUi()
  const color =
    variant === 'primary'
      ? theme.background
      : variant === 'destructive'
        ? theme.destructive
        : theme.foreground
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
        gap: 6,
        height: small ? scale.buttonXsHeight + 4 : scale.buttonHeight + 4,
        paddingLeft: small ? 8 : 12,
        paddingRight: small ? 8 : 12,
        borderRadius: 6,
        borderWidth: variant === 'outline' ? 1 : 0,
        borderColor: theme.border,
        backgroundColor: variant === 'primary' ? theme.foreground : undefined,
        cursor: 'pointer',
        flexShrink: 0,
        userSelect: 'none',
        hover: {
          backgroundColor: variant === 'primary' ? theme.mutedForeground : theme.sidebarAccent,
        },
      }}
    >
      {icon ? <Icon name={icon} color={color} /> : null}
      <Label color={color} size={small ? scale.controlFont : undefined} weight={500}>
        {children}
      </Label>
    </div>
  )
}

export function Kbd({ children }: { children: string }) {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        paddingLeft: 6,
        paddingRight: 6,
        paddingTop: 1,
        paddingBottom: 1,
        borderRadius: 4,
        borderWidth: 1,
        borderColor: theme.border,
        backgroundColor: theme.muted,
        flexShrink: 0,
      }}
    >
      <Label size={scale.controlXsFont + 1} color={theme.mutedForeground}>
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

export function EmptyState({
  icon,
  title,
  body,
  detail,
  children,
}: {
  icon: IconName
  title: string
  body: string
  detail?: string
  children?: ReactNode
}) {
  const { theme, scale } = useUi()
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
