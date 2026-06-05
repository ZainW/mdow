import { useId } from 'react'
import { Sun, Moon, Monitor } from 'lucide-react'
import { useAppStore } from '../store/app-store'
import {
  CODE_FONTS,
  CONTENT_FONTS,
  MARKDOWN_FONT_SIZE,
  MARKDOWN_LINE_HEIGHT,
  getCodeFontFamily,
  getContentFontFamily,
} from '../lib/typography'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from './ui/dialog'
import { Button } from './ui/button'
import { Input } from './ui/input'
import { Label } from './ui/label'
import { Switch } from './ui/switch'
import { ToggleGroup, ToggleGroupItem } from './ui/toggle-group'
import { cn } from '@renderer/lib/utils'
import { rovingTabIndex, useRovingFocus } from '../hooks/useRovingFocus'
import { iconActiveProps } from '../lib/icons'
import type { CompanionProviderId, InterfaceScale, ReadingWidth } from '../../../shared/types'

const DEFAULTS = {
  theme: 'system' as const,
  contentFont: 'inter',
  codeFont: 'geist-mono',
  interfaceScale: 'compact' as const,
  readingWidth: 'standard' as const,
  autoUpdateEnabled: true,
  companionProvider: 'auto' as const,
  companionCustomCommand: '',
}

const THEME_OPTIONS = [
  { value: 'system', label: 'System', Icon: Monitor },
  { value: 'light', label: 'Light', Icon: Sun },
  { value: 'dark', label: 'Dark', Icon: Moon },
] as const

const INTERFACE_SCALE_OPTIONS = [
  { value: 'compact', label: 'Compact' },
  { value: 'comfortable', label: 'Comfortable' },
  { value: 'large', label: 'Large' },
] as const satisfies readonly { value: InterfaceScale; label: string }[]

const READING_WIDTH_OPTIONS = [
  { value: 'standard', label: 'Standard' },
  { value: 'comfortable', label: 'Comfortable' },
  { value: 'wide', label: 'Wide' },
] as const satisfies readonly { value: ReadingWidth; label: string }[]

const COMPANION_PROVIDER_OPTIONS = [
  { value: 'auto', label: 'Auto' },
  { value: 'opencode', label: 'opencode' },
  { value: 'codex', label: 'Codex' },
  { value: 'custom', label: 'Custom' },
] as const satisfies readonly { value: CompanionProviderId; label: string }[]

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const contentFont = useAppStore((s) => s.contentFont)
  const codeFont = useAppStore((s) => s.codeFont)
  const theme = useAppStore((s) => s.theme)
  const interfaceScale = useAppStore((s) => s.interfaceScale)
  const readingWidth = useAppStore((s) => s.readingWidth)
  const setContentFont = useAppStore((s) => s.setContentFont)
  const setCodeFont = useAppStore((s) => s.setCodeFont)
  const setTheme = useAppStore((s) => s.setTheme)
  const setInterfaceScale = useAppStore((s) => s.setInterfaceScale)
  const setReadingWidth = useAppStore((s) => s.setReadingWidth)
  const autoUpdateEnabled = useAppStore((s) => s.autoUpdateEnabled)
  const setAutoUpdateEnabled = useAppStore((s) => s.setAutoUpdateEnabled)
  const companionProvider = useAppStore((s) => s.companionProvider)
  const companionCustomCommand = useAppStore((s) => s.companionCustomCommand)
  const setCompanionProvider = useAppStore((s) => s.setCompanionProvider)
  const setCompanionCustomCommand = useAppStore((s) => s.setCompanionCustomCommand)
  const setCompanionProviders = useAppStore((s) => s.setCompanionProviders)
  const setCompanionError = useAppStore((s) => s.setCompanionError)

  const contentFamily = getContentFontFamily(contentFont)
  const codeFamily = getCodeFontFamily(codeFont)

  const handleResetDefaults = () => {
    setTheme(DEFAULTS.theme)
    setContentFont(DEFAULTS.contentFont)
    setCodeFont(DEFAULTS.codeFont)
    setInterfaceScale(DEFAULTS.interfaceScale)
    setReadingWidth(DEFAULTS.readingWidth)
    setAutoUpdateEnabled(DEFAULTS.autoUpdateEnabled)
    setCompanionProvider(DEFAULTS.companionProvider)
    setCompanionCustomCommand(DEFAULTS.companionCustomCommand)
    void persistCompanionSettings(DEFAULTS.companionProvider, DEFAULTS.companionCustomCommand)
  }

  const persistCompanionSettings = async (provider: CompanionProviderId, customCommand: string) => {
    try {
      await window.api.saveCompanionSettings({ provider, customCommand })
      const providers = await window.api.detectCompanionProviders()
      setCompanionProviders(providers)
      setCompanionError(null)
    } catch (error) {
      setCompanionError(error instanceof Error ? error.message : String(error))
    }
  }

  const handleCompanionProviderChange = (provider: CompanionProviderId) => {
    setCompanionProvider(provider)
    void persistCompanionSettings(provider, companionCustomCommand)
  }

  const handleCompanionCustomCommandChange = (command: string) => {
    setCompanionCustomCommand(command)
    void persistCompanionSettings(companionProvider, command)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100vh-2rem)] gap-5 overflow-y-auto sm:max-w-md">
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
            style={{ fontSize: `${MARKDOWN_FONT_SIZE * 1.25}px`, lineHeight: 1.25 }}
          >
            The quiet morning
          </div>
          <p
            className="mt-1.5 text-foreground/85"
            style={{ fontSize: `${MARKDOWN_FONT_SIZE}px`, lineHeight: MARKDOWN_LINE_HEIGHT }}
          >
            Words on the page settle into their rhythm, and{' '}
            <code
              className="rounded bg-muted px-1 py-px text-foreground"
              style={{
                fontFamily: codeFamily,
                fontSize: `${MARKDOWN_FONT_SIZE * 0.9}px`,
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

        <Field label="Interface scale">
          <PresetToggleGroup
            groupLabel="Interface scale"
            value={interfaceScale}
            options={INTERFACE_SCALE_OPTIONS}
            onChange={setInterfaceScale}
          />
        </Field>

        <Field label="Reading width">
          <PresetToggleGroup
            groupLabel="Reading width"
            value={readingWidth}
            options={READING_WIDTH_OPTIONS}
            onChange={setReadingWidth}
          />
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

        <section className="space-y-3">
          <div className="space-y-1">
            <h3 className="text-sm font-medium">Companion</h3>
            <p className="text-sm text-muted-foreground">
              Custom commands run as local subprocesses and should point to an ACP-compatible agent.
            </p>
          </div>
          <Field label="Provider">
            <PresetToggleGroup
              groupLabel="Companion provider"
              value={companionProvider}
              options={COMPANION_PROVIDER_OPTIONS}
              onChange={handleCompanionProviderChange}
            />
          </Field>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="settings-companion-custom-command" className="font-medium">
              Custom ACP command
            </Label>
            <Input
              id="settings-companion-custom-command"
              aria-label="Custom ACP command"
              value={companionCustomCommand}
              onChange={(event) => handleCompanionCustomCommandChange(event.currentTarget.value)}
              placeholder="custom-acp --stdio"
            />
          </div>
        </section>

        <div className="flex justify-end border-t border-border-subtle pt-3">
          <Button variant="outline" size="sm" onClick={handleResetDefaults}>
            Reset to defaults
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}

function PresetToggleGroup<TValue extends string>({
  groupLabel,
  value,
  options,
  onChange,
}: {
  groupLabel: string
  value: TValue
  options: readonly { value: TValue; label: string }[]
  onChange: (value: TValue) => void
}) {
  return (
    <ToggleGroup
      aria-label={groupLabel}
      value={[value]}
      onValueChange={(nextValue) => {
        const next = options.find((opt) => opt.value === nextValue[0])?.value
        if (next) onChange(next)
      }}
      variant="outline"
      spacing={0}
      className={cn(
        'grid w-full rounded-md bg-muted p-0.5',
        options.length === 4 ? 'grid-cols-4' : 'grid-cols-3',
      )}
    >
      {options.map((opt) => (
        <ToggleGroupItem
          key={opt.value}
          value={opt.value}
          aria-label={opt.label}
          className="flex-1 rounded-[5px] data-pressed:bg-background data-pressed:text-foreground data-pressed:shadow-sm data-pressed:ring-1 data-pressed:ring-foreground/10 dark:data-pressed:ring-foreground/15"
        >
          {opt.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
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
        const next = THEME_OPTIONS.find((opt) => opt.value === value[0])?.value
        if (next) onChange(next)
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
            'flex-1 gap-1.5 rounded-[5px] data-pressed:bg-background data-pressed:text-foreground data-pressed:shadow-sm data-pressed:ring-1 data-pressed:ring-foreground/10 dark:data-pressed:ring-foreground/15',
          )}
        >
          <opt.Icon
            className="size-(--button-default-icon-size)"
            {...iconActiveProps(theme === opt.value)}
          />
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
      <Label id={id} className="font-medium text-muted-foreground">
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
