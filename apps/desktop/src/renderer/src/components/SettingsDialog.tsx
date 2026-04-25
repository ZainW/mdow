import { Sun, Moon, Desktop } from '@phosphor-icons/react'
import { useAppStore } from '../store/app-store'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from './ui/dialog'
import { Slider } from './ui/slider'
import { cn } from '@renderer/lib/utils'

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

  const contentFamily = getContentFontFamily(contentFont)
  const codeFamily = getCodeFontFamily(codeFont)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="gap-5 sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>Tune how markdown reads.</DialogDescription>
        </DialogHeader>

        {/* ── Live preview ────────────────────────────────────── */}
        <div
          className="overflow-hidden rounded-lg border border-border-subtle bg-muted/40 px-4 py-3.5"
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

        {/* ── Theme ───────────────────────────────────────────── */}
        <Field label="Theme">
          <div className="grid grid-cols-3 gap-1 rounded-md bg-muted p-0.5">
            {THEME_OPTIONS.map((opt) => {
              const active = theme === opt.value
              return (
                <button
                  key={opt.value}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setTheme(opt.value)}
                  className={cn(
                    'flex h-7 items-center justify-center gap-1.5 rounded-[5px] text-xs font-medium transition-colors outline-none',
                    'focus-visible:ring-2 focus-visible:ring-ring/50',
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

        {/* ── Content font ────────────────────────────────────── */}
        <Field label="Content font">
          <div className="grid grid-cols-4 gap-1.5">
            {CONTENT_FONTS.map((font) => {
              const active = contentFont === font.value
              return (
                <FontTile
                  key={font.value}
                  active={active}
                  label={font.label}
                  family={font.family}
                  glyph="Aa"
                  onClick={() => setContentFont(font.value)}
                />
              )
            })}
          </div>
        </Field>

        {/* ── Code font ───────────────────────────────────────── */}
        <Field label="Code font">
          <div className="grid grid-cols-3 gap-1.5">
            {CODE_FONTS.map((font) => {
              const active = codeFont === font.value
              return (
                <FontTile
                  key={font.value}
                  active={active}
                  label={font.label}
                  family={font.family}
                  glyph="() => {}"
                  glyphSize={13}
                  onClick={() => setCodeFont(font.value)}
                />
              )
            })}
          </div>
        </Field>

        {/* ── Sliders ─────────────────────────────────────────── */}
        <div className="grid grid-cols-1 gap-4">
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
            valueLabel={lineHeight.toFixed(2)}
            value={lineHeight}
            onValueChange={(v) => setLineHeight(Number(v))}
            min={1.2}
            max={2.2}
            step={0.1}
            leftHint="Compact"
            rightHint="Spacious"
          />
        </div>
      </DialogContent>
    </Dialog>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
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
        'group flex flex-col items-center justify-center gap-1.5 rounded-md border px-2 py-2.5 transition-colors outline-none',
        'focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40',
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
        style={{ fontFamily: family, fontSize: glyphSize ? `${glyphSize}px` : '18px' }}
      >
        {glyph}
      </span>
      <span
        className={cn(
          'text-[10px] leading-none tracking-wide',
          active ? 'font-medium text-foreground' : 'text-muted-foreground',
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
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between">
        <span className="text-xs font-medium text-muted-foreground">{label}</span>
        <span className="text-xs tabular-nums text-foreground">{valueLabel}</span>
      </div>
      <div className="flex items-center gap-3">
        <span
          className="select-none text-muted-foreground/70"
          style={leftHintSize ? { fontSize: `${leftHintSize}px` } : { fontSize: '10px' }}
        >
          {leftHint}
        </span>
        <Slider
          value={value}
          onValueChange={onValueChange}
          min={min}
          max={max}
          step={step}
          className="flex-1"
        />
        <span
          className="select-none text-muted-foreground/70"
          style={rightHintSize ? { fontSize: `${rightHintSize}px` } : { fontSize: '10px' }}
        >
          {rightHint}
        </span>
      </div>
    </div>
  )
}
