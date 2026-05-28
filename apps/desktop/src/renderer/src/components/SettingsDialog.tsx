import { useId } from 'react'
import { Sun, Moon, Monitor } from 'lucide-react'
import { useAppStore } from '../store/app-store'
import {
  CODE_FONTS,
  CONTENT_FONTS,
  getCodeFontFamily,
  getContentFontFamily,
} from '../lib/typography'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from './ui/dialog'
import { Slider } from './ui/slider'
import { Button } from './ui/button'
import { Label } from './ui/label'
import { Switch } from './ui/switch'
import { ToggleGroup, ToggleGroupItem } from './ui/toggle-group'
import { cn, isMac } from '@renderer/lib/utils'
import { rovingTabIndex, useRovingFocus } from '../hooks/useRovingFocus'
import { iconActiveProps } from '../lib/icons'

const DEFAULTS = {
  theme: 'system' as const,
  contentFont: 'inter',
  codeFont: 'geist-mono',
  fontSize: 15.5,
  lineHeight: 1.65,
  autoUpdateEnabled: true,
}

const THEME_OPTIONS = [
  { value: 'system', label: 'System', Icon: Monitor },
  { value: 'light', label: 'Light', Icon: Sun },
  { value: 'dark', label: 'Dark', Icon: Moon },
] as const

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

  const handleResetDefaults = () => {
    setTheme(DEFAULTS.theme)
    setContentFont(DEFAULTS.contentFont)
    setCodeFont(DEFAULTS.codeFont)
    setFontSize(DEFAULTS.fontSize)
    setLineHeight(DEFAULTS.lineHeight)
    if (!isMac) setAutoUpdateEnabled(DEFAULTS.autoUpdateEnabled)
  }

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
            Words on the page settle into their rhythm, and{' '}
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
          <ThemeRadiogroup theme={theme} onChange={setTheme} />
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
            <div className="flex items-center justify-between gap-3 text-sm">
              <span className="text-muted-foreground">
                Automatically check for updates in the background
              </span>
              <Switch
                checked={autoUpdateEnabled}
                onCheckedChange={setAutoUpdateEnabled}
                aria-label="Automatically check for updates in the background"
              />
            </div>
          </section>
        )}

        <p className="text-[11px] leading-snug text-muted-foreground/70">
          Wide mode is toggled from the document toolbar and saved across sessions.
        </p>

        <div className="flex justify-end border-t border-border-subtle pt-3">
          <Button variant="outline" size="sm" onClick={handleResetDefaults}>
            Reset to defaults
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}

function ThemeRadiogroup({
  theme,
  onChange,
}: {
  theme: string
  onChange: (value: 'system' | 'light' | 'dark') => void
}) {
  return (
    <ToggleGroup
      aria-label="Theme"
      value={[theme]}
      onValueChange={(value) => {
        const next = value[0]
        if (next) onChange(next as 'system' | 'light' | 'dark')
      }}
      variant="outline"
      spacing={0}
      className="grid w-full grid-cols-3 rounded-md bg-muted p-0.5"
    >
      {THEME_OPTIONS.map((opt) => (
        <ToggleGroupItem
          key={opt.value}
          value={opt.value}
          aria-label={opt.label}
          className={cn(
            'h-7 flex-1 gap-1.5 rounded-[5px] text-xs data-pressed:bg-background data-pressed:text-foreground data-pressed:shadow-sm data-pressed:ring-1 data-pressed:ring-foreground/10 dark:data-pressed:ring-foreground/15',
          )}
        >
          <opt.Icon className="size-3.5" {...iconActiveProps(theme === opt.value)} />
          {opt.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  const id = useId()
  return (
    <div className="flex flex-col gap-1.5">
      <Label id={id} className="text-xs font-medium text-muted-foreground">
        {label}
      </Label>
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
  const { containerRef, onKeyDown } = useRovingFocus({ orientation: 'horizontal' })
  return (
    // oxlint-disable-next-line jsx-a11y/interactive-supports-focus -- per WAI-ARIA, focus rests on the active radio inside, not the radiogroup itself
    <div
      ref={containerRef}
      role="radiogroup"
      aria-label={groupLabel}
      onKeyDown={onKeyDown}
      className={cn('m-0 grid min-w-0 gap-1.5', cols === 3 ? 'grid-cols-3' : 'grid-cols-4')}
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
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- custom-styled font tile, native radio input would break layout
      role="radio"
      tabIndex={rovingTabIndex(active)}
      aria-checked={active}
      onClick={onClick}
      className={cn(
        'group relative flex cursor-pointer flex-col items-center justify-center gap-1.5 rounded-md border px-2 py-2.5 outline-none',
        'transition-[background-color,border-color,box-shadow,transform] duration-150',
        'active:scale-[0.98] focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40',
        active
          ? 'border-foreground/25 bg-accent/10 ring-1 ring-foreground/10 dark:border-foreground/30 dark:bg-accent/15 dark:ring-foreground/20'
          : 'border-border-subtle bg-background hover:border-border hover:bg-muted/60',
      )}
    >
      {active && (
        <span aria-hidden className="absolute top-1.5 right-1.5 size-1.5 rounded-full bg-primary" />
      )}
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
        <Label htmlFor={id} className="text-xs font-medium text-muted-foreground">
          {label}
        </Label>
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
