import { useId } from 'react'
import { Sun, Moon, Desktop } from '@phosphor-icons/react'
import { useAppStore } from '../store/app-store'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from './ui/dialog'
import { Slider } from './ui/slider'
import { cn, isMac } from '@renderer/lib/utils'

const CONTENT_FONTS = [
  { value: 'inter', label: 'Inter', family: "'Inter', system-ui, -apple-system, sans-serif" },
  { value: 'charter', label: 'Charter', family: "Charter, 'Bitstream Charter', Georgia, serif" },
  {
    value: 'system-sans',
    label: 'System',
    family: 'system-ui, -apple-system, sans-serif',
  },
  { value: 'georgia', label: 'Georgia', family: "Georgia, 'Times New Roman', serif" },
] as const

const CODE_FONTS = [
  { value: 'geist-mono', label: 'Geist', family: "'Geist Mono', ui-monospace, monospace" },
  {
    value: 'sf-mono',
    label: 'SF Mono',
    family: "'SF Mono', SFMono-Regular, ui-monospace, monospace",
  },
  {
    value: 'jetbrains-mono',
    label: 'JetBrains',
    family: "'JetBrains Mono', ui-monospace, monospace",
  },
] as const

const THEME_OPTIONS = [
  { value: 'system', label: 'System', Icon: Desktop },
  { value: 'light', label: 'Light', Icon: Sun },
  { value: 'dark', label: 'Dark', Icon: Moon },
] as const

export function getContentFontFamily(value: string): string {
  return CONTENT_FONTS.find((f) => f.value === value)?.family ?? CONTENT_FONTS[0].family
}

export function getCodeFontFamily(value: string): string {
  return CODE_FONTS.find((f) => f.value === value)?.family ?? CODE_FONTS[0].family
}

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const contentFont = useAppStore((s) => s.contentFont)
  const codeFont = useAppStore((s) => s.codeFont)
  const fontSize = useAppStore((s) => s.fontSize)
  const lineHeight = useAppStore((s) => s.lineHeight)
  const theme = useAppStore((s) => s.theme)
  const setContentFont = useAppStore((s) => s.setContentFont)
  const setCodeFont = useAppStore((s) => s.setCodeFont)
  const setFontSize = useAppStore((s) => s.setFontSize)
  const setLineHeight = useAppStore((s) => s.setLineHeight)
  const setTheme = useAppStore((s) => s.setTheme)
  const autoUpdateEnabled = useAppStore((s) => s.autoUpdateEnabled)
  const setAutoUpdateEnabled = useAppStore((s) => s.setAutoUpdateEnabled)

  const contentFamily = getContentFontFamily(contentFont)
  const codeFamily = getCodeFontFamily(codeFont)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="gap-5 sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>Tune how markdown reads.</DialogDescription>
        </DialogHeader>

        {/* Decorative preview — not in tab/select flow */}
        <div
          className="overflow-hidden rounded-lg border border-border-subtle bg-muted/40 px-4 py-3.5 select-none [&_*]:pointer-events-none"
          style={{ fontFamily: contentFamily }}
          aria-hidden="true"
        >
          <div
            className="font-semibold tracking-tight text-foreground"
            style={{ fontSize: `${fontSize * 1.25}px`, lineHeight: 1.25 }}
          >
            The quiet morning
          </div>
          <p
            className="mt-1.5 text-foreground/85"
            style={{ fontSize: `${fontSize}px`, lineHeight }}
          >
            Words on the page settle into their rhythm — and{' '}
            <code
              className="rounded bg-muted px-1 py-px text-foreground"
              style={{
                fontFamily: codeFamily,
                fontSize: `${fontSize * 0.9}px`,
              }}
            >
              ligatures
            </code>{' '}
            too.
          </p>
        </div>

        <Field label="Theme">
          <div
            role="group"
            aria-label="Theme"
            className="grid grid-cols-3 gap-1 rounded-md bg-muted p-0.5"
          >
            {THEME_OPTIONS.map((opt) => {
              const active = theme === opt.value
              return (
                <button
                  key={opt.value}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setTheme(opt.value)}
                  className={cn(
                    'flex h-7 cursor-pointer items-center justify-center gap-1.5 rounded-[5px] text-xs font-medium outline-none',
                    'transition-[background-color,color,box-shadow,transform] duration-150',
                    'active:scale-[0.97] focus-visible:ring-2 focus-visible:ring-ring/50',
                    active
                      ? 'bg-background text-foreground shadow-sm ring-1 ring-foreground/5'
                      : 'text-muted-foreground hover:text-foreground',
                  )}
                >
                  <opt.Icon weight={active ? 'fill' : 'regular'} className="size-3.5" />
                  {opt.label}
                </button>
              )
            })}
          </div>
        </Field>

        <Field label="Content font">
          <FontGrid groupLabel="Content font" cols={4}>
            {CONTENT_FONTS.map((font) => (
              <FontTile
                key={font.value}
                active={contentFont === font.value}
                label={font.label}
                family={font.family}
                glyph="Aa"
                onClick={() => setContentFont(font.value)}
              />
            ))}
          </FontGrid>
        </Field>

        <Field label="Code font">
          <FontGrid groupLabel="Code font" cols={3}>
            {CODE_FONTS.map((font) => (
              <FontTile
                key={font.value}
                active={codeFont === font.value}
                label={font.label}
                family={font.family}
                glyph="() => {}"
                glyphSize={13}
                onClick={() => setCodeFont(font.value)}
              />
            ))}
          </FontGrid>
        </Field>

        <div className="flex flex-col gap-4">
          <SliderField
            label="Size"
            valueLabel={`${fontSize}px`}
            value={fontSize}
            onValueChange={(v) => setFontSize(Number(v))}
            min={13}
            max={24}
            step={1}
            leftHint="Aa"
            rightHint="Aa"
            leftHintSize={12}
            rightHintSize={18}
          />
          <SliderField
            label="Line height"
            valueLabel={lineHeight.toFixed(1)}
            value={lineHeight}
            onValueChange={(v) => setLineHeight(Number(v))}
            min={1.2}
            max={2.2}
            step={0.1}
            leftHint="Compact"
            rightHint="Spacious"
          />
        </div>

        {!isMac && (
          <section className="space-y-2">
            <h3 className="text-sm font-medium">Updates</h3>
            <label className="flex items-center justify-between gap-3 text-sm">
              <span className="text-muted-foreground">
                Automatically check for updates in the background
              </span>
              <input
                type="checkbox"
                checked={autoUpdateEnabled}
                onChange={(e) => setAutoUpdateEnabled(e.target.checked)}
                className="size-4 cursor-pointer accent-primary"
              />
            </label>
          </section>
        )}
      </DialogContent>
    </Dialog>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  const id = useId()
  return (
    <div className="flex flex-col gap-1.5">
      <div id={id} className="text-xs font-medium text-muted-foreground">
        {label}
      </div>
      {children}
    </div>
  )
}

function FontGrid({
  groupLabel,
  cols,
  children,
}: {
  groupLabel: string
  cols: 3 | 4
  children: React.ReactNode
}) {
  return (
    <div
      role="group"
      aria-label={groupLabel}
      className={cn('grid gap-1.5', cols === 3 ? 'grid-cols-3' : 'grid-cols-4')}
    >
      {children}
    </div>
  )
}

function FontTile({
  active,
  label,
  family,
  glyph,
  glyphSize,
  onClick,
}: {
  active: boolean
  label: string
  family: string
  glyph: string
  glyphSize?: number
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={cn(
        'group flex cursor-pointer flex-col items-center justify-center gap-1.5 rounded-md border px-2 py-2.5 outline-none',
        'transition-[background-color,border-color,box-shadow,transform] duration-150',
        'active:scale-[0.98] focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40',
        active
          ? 'border-foreground/15 bg-accent/5 ring-1 ring-foreground/5'
          : 'border-border-subtle bg-background hover:border-border hover:bg-muted/60',
      )}
    >
      <span
        className={cn(
          'leading-none',
          active ? 'text-foreground' : 'text-foreground/70 group-hover:text-foreground',
        )}
        style={{
          fontFamily: family,
          fontSize: glyphSize ? `${glyphSize / 16}rem` : '1.125rem',
        }}
      >
        {glyph}
      </span>
      <span
        className={cn(
          'text-[0.625rem] font-medium leading-none tracking-wide',
          active ? 'text-foreground' : 'text-muted-foreground',
        )}
      >
        {label}
      </span>
    </button>
  )
}

interface SliderFieldProps {
  label: string
  valueLabel: string
  value: number
  onValueChange: (value: number | readonly number[]) => void
  min: number
  max: number
  step: number
  leftHint: string
  rightHint: string
  leftHintSize?: number
  rightHintSize?: number
}

function SliderField({
  label,
  valueLabel,
  value,
  onValueChange,
  min,
  max,
  step,
  leftHint,
  rightHint,
  leftHintSize,
  rightHintSize,
}: SliderFieldProps) {
  const id = useId()
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between">
        <label htmlFor={id} className="text-xs font-medium text-muted-foreground">
          {label}
        </label>
        <span className="text-xs tabular-nums text-foreground">{valueLabel}</span>
      </div>
      <div className="flex items-center gap-3">
        <span
          aria-hidden="true"
          className="select-none leading-none text-muted-foreground/70"
          style={{ fontSize: leftHintSize ? `${leftHintSize / 16}rem` : '0.625rem' }}
        >
          {leftHint}
        </span>
        <Slider
          id={id}
          aria-label={label}
          value={value}
          onValueChange={onValueChange}
          min={min}
          max={max}
          step={step}
          className="flex-1"
        />
        <span
          aria-hidden="true"
          className="select-none leading-none text-muted-foreground/70"
          style={{ fontSize: rightHintSize ? `${rightHintSize / 16}rem` : '0.625rem' }}
        >
          {rightHint}
        </span>
      </div>
    </div>
  )
}
