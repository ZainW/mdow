import { useEffect, useId, useState } from 'react'
import { Sun, Moon, Monitor, Minus, Plus, RotateCcw } from 'lucide-react'
import { useAppStore } from '../store/app-store'
import { ZOOM_MAX, ZOOM_MIN } from '../store/slices/settings-slice'
import {
  CODE_FONTS,
  CONTENT_FONTS,
  MARKDOWN_LINE_HEIGHT,
  getCodeFontFamily,
  getContentFontFamily,
} from '../lib/typography'
import { Dialog, DialogContent, DialogTitle, DialogDescription } from './ui/dialog'
import { Button } from './ui/button'
import { Switch } from './ui/switch'
import { SegmentedControl, type SegmentedOption } from './SegmentedControl'
import { cn, isMac } from '../lib/utils'
import type { InterfaceScale, ReadingWidth, CompanionProviderId } from '../../../shared/types'

const DEFAULTS = {
  theme: 'system' as const,
  contentFont: 'inter',
  codeFont: 'geist-mono',
  interfaceScale: 'compact' as const,
  readingWidth: 'medium' as const,
  autoUpdateEnabled: true,
}

const PROVIDER_OPTIONS = [
  { value: 'opencode', label: 'OpenCode' },
  { value: 'codex-acp', label: 'Codex ACP' },
  { value: 'custom', label: 'Custom' },
] as const satisfies readonly SegmentedOption<CompanionProviderId>[]

const THEME_OPTIONS = [
  { value: 'system', label: 'System', Icon: Monitor },
  { value: 'light', label: 'Light', Icon: Sun },
  { value: 'dark', label: 'Dark', Icon: Moon },
] as const satisfies readonly SegmentedOption<'system' | 'light' | 'dark'>[]

const INTERFACE_SCALE_OPTIONS = [
  { value: 'compact', label: 'Compact' },
  { value: 'comfortable', label: 'Comfortable' },
  { value: 'large', label: 'Large' },
] as const satisfies readonly SegmentedOption<InterfaceScale>[]

const LINE_WIDTH_OPTIONS = [
  { value: 'narrow', label: 'Narrow' },
  { value: 'medium', label: 'Medium' },
  { value: 'wide', label: 'Wide' },
  { value: 'full', label: 'Full' },
] as const satisfies readonly SegmentedOption<ReadingWidth>[]

// Each font option previews in its own typeface.
const TEXT_FONT_OPTIONS: SegmentedOption<string>[] = CONTENT_FONTS.map((font) => ({
  value: font.value,
  label: font.label,
  style: { fontFamily: font.family, fontWeight: 400 },
}))
const CODE_FONT_OPTIONS: SegmentedOption<string>[] = CODE_FONTS.map((font) => ({
  value: font.value,
  label: font.label,
  style: { fontFamily: font.family },
}))

const mod = isMac ? '⌘' : 'Ctrl+'

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
  const zoomLevel = useAppStore((s) => s.zoomLevel)
  const setContentFont = useAppStore((s) => s.setContentFont)
  const setCodeFont = useAppStore((s) => s.setCodeFont)
  const setTheme = useAppStore((s) => s.setTheme)
  const setInterfaceScale = useAppStore((s) => s.setInterfaceScale)
  const setReadingWidth = useAppStore((s) => s.setReadingWidth)
  const zoomIn = useAppStore((s) => s.zoomIn)
  const zoomOut = useAppStore((s) => s.zoomOut)
  const resetZoom = useAppStore((s) => s.resetZoom)
  const autoUpdateEnabled = useAppStore((s) => s.autoUpdateEnabled)
  const setAutoUpdateEnabled = useAppStore((s) => s.setAutoUpdateEnabled)
  const companionPreferredProvider = useAppStore((s) => s.companionPreferredProvider)
  const setCompanionPreferredProvider = useAppStore((s) => s.setCompanionPreferredProvider)
  const companionCustomCommand = useAppStore((s) => s.companionCustomCommand)
  const setCompanionCustomCommand = useAppStore((s) => s.setCompanionCustomCommand)

  const chooseCompanionExecutable = async () => {
    const executablePath = await window.api.chooseCompanionCustomExecutable()
    if (!executablePath) return
    setCompanionCustomCommand(executablePath)
    setCompanionPreferredProvider('custom')
    const providers = await window.api.detectCompanionProviders()
    useAppStore.getState().setCompanionProviders(providers)
  }

  const handleResetDefaults = () => {
    setTheme(DEFAULTS.theme)
    setContentFont(DEFAULTS.contentFont)
    setCodeFont(DEFAULTS.codeFont)
    setInterfaceScale(DEFAULTS.interfaceScale)
    setReadingWidth(DEFAULTS.readingWidth)
    resetZoom()
    setAutoUpdateEnabled(DEFAULTS.autoUpdateEnabled)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="settings-dialog max-h-[calc(100dvh-3rem)] gap-0 overflow-y-auto overscroll-contain px-5 pt-[18px] pb-4 text-[13px] sm:max-w-[520px]">
        <DialogTitle className="text-[15px] font-semibold">Settings</DialogTitle>
        <DialogDescription className="sr-only">
          Appearance, reading and update preferences. Changes save automatically.
        </DialogDescription>

        <ReadingPreview contentFont={contentFont} codeFont={codeFont} zoomLevel={zoomLevel} />

        <SettingsGroup title="Appearance">
          <SettingsRow label="Theme">
            <SegmentedControl
              label="Theme"
              value={theme === 'light' || theme === 'dark' ? theme : 'system'}
              options={THEME_OPTIONS}
              onChange={setTheme}
              className={SEGMENTED_CLASS}
            />
          </SettingsRow>
          <SettingsRow label="Interface size">
            <SegmentedControl
              label="Interface size"
              value={interfaceScale}
              options={INTERFACE_SCALE_OPTIONS}
              onChange={setInterfaceScale}
              className={SEGMENTED_CLASS}
            />
          </SettingsRow>
        </SettingsGroup>

        <SettingsGroup title="Reading">
          <SettingsRow label="Text font">
            <SegmentedControl
              label="Text font"
              value={contentFont}
              options={TEXT_FONT_OPTIONS}
              onChange={setContentFont}
              className={SEGMENTED_CLASS}
            />
          </SettingsRow>
          <SettingsRow label="Code font">
            <SegmentedControl
              label="Code font"
              value={codeFont}
              options={CODE_FONT_OPTIONS}
              onChange={setCodeFont}
              className={SEGMENTED_CLASS}
            />
          </SettingsRow>
          <SettingsRow label="Line width">
            <SegmentedControl
              label="Line width"
              value={readingWidth}
              options={LINE_WIDTH_OPTIONS}
              onChange={setReadingWidth}
              className={SEGMENTED_CLASS}
            />
          </SettingsRow>
          <SettingsRow label="Text size" hint={`${mod}+ / ${mod}−`}>
            <TextSizeStepper
              value={zoomLevel}
              onDecrease={zoomOut}
              onIncrease={zoomIn}
              onReset={resetZoom}
            />
          </SettingsRow>
        </SettingsGroup>

        <SettingsGroup title="Companion">
          <SettingsRow label="Provider" hint="Local ACP agents only">
            <SegmentedControl
              label="Companion provider"
              value={companionPreferredProvider ?? 'opencode'}
              options={PROVIDER_OPTIONS}
              onChange={setCompanionPreferredProvider}
              className={SEGMENTED_CLASS}
            />
          </SettingsRow>
          {companionPreferredProvider === 'custom' && (
            <SettingsRow label="Executable">
              <div className="flex min-w-0 items-center gap-2">
                <span
                  className="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground"
                  title={companionCustomCommand || undefined}
                >
                  {companionCustomCommand || 'None chosen'}
                </span>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void chooseCompanionExecutable()}
                >
                  Choose…
                </Button>
              </div>
            </SettingsRow>
          )}
          <p className="pb-1 pl-[136px] text-[11.5px] text-muted-foreground">
            OpenCode Go is configured inside OpenCode, not here.
          </p>
        </SettingsGroup>

        <SettingsGroup title="Updates">
          <UpdatesRow />
          <SettingsRow label="Automatic">
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-muted-foreground">
                Check for updates in the background
              </span>
              <Switch
                checked={autoUpdateEnabled}
                onCheckedChange={setAutoUpdateEnabled}
                aria-label="Automatically check for updates in the background"
              />
            </div>
          </SettingsRow>
        </SettingsGroup>

        <div className="mt-3.5 flex items-center border-t border-border-subtle pt-3">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="-ml-2 gap-1.5 text-xs font-normal text-muted-foreground hover:text-foreground"
            onClick={handleResetDefaults}
          >
            <RotateCcw className="size-3" aria-hidden />
            Restore defaults
          </Button>
          <span className="ml-auto text-[11.5px] text-muted-foreground">
            Changes save automatically
          </span>
        </div>
      </DialogContent>
    </Dialog>
  )
}

const SEGMENTED_CLASS = 'h-[30px] w-full'

function SettingsGroup({ title, children }: { title: string; children: React.ReactNode }) {
  const id = useId()
  return (
    <section aria-labelledby={id}>
      <h3
        id={id}
        className="mt-3.5 mb-1 text-[11px] font-semibold tracking-[0.04em] text-muted-foreground uppercase"
      >
        {title}
      </h3>
      <div className="border-t border-border-subtle pt-1.5">{children}</div>
    </section>
  )
}

function SettingsRow({
  label,
  hint,
  children,
}: {
  label: string
  hint?: string
  children: React.ReactNode
}) {
  return (
    <div className="flex min-h-10 items-center gap-4">
      <div className="w-[120px] shrink-0">
        <div className="text-[13px] text-foreground">{label}</div>
        {hint && <div className="mt-px text-[11.5px] text-muted-foreground">{hint}</div>}
      </div>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  )
}

function ReadingPreview({
  contentFont,
  codeFont,
  zoomLevel,
}: {
  contentFont: string
  codeFont: string
  zoomLevel: number
}) {
  const scale = zoomLevel / 100
  return (
    // Decorative: a real reader sample on the page background, in the chosen fonts and size.
    <div
      aria-hidden
      className="mt-3.5 overflow-hidden rounded-lg border border-border-subtle bg-background px-4 py-3.5 select-none"
    >
      <div
        className="font-semibold text-foreground"
        style={{
          fontFamily: getContentFontFamily(contentFont),
          fontSize: `${19 * scale}px`,
          lineHeight: 1.3,
        }}
      >
        A quiet place to read
      </div>
      <p
        className="mt-1 text-foreground"
        style={{
          fontFamily: getContentFontFamily(contentFont),
          fontSize: `${16 * scale}px`,
          lineHeight: MARKDOWN_LINE_HEIGHT,
        }}
      >
        Mdow re-renders the moment you save, so notes stay live beside your editor.
      </p>
      <div
        className="mt-1.5 text-muted-foreground"
        style={{ fontFamily: getCodeFontFamily(codeFont), fontSize: `${13 * scale}px` }}
      >
        <span className="text-(--md-alert-caution)">let</span> width ={' '}
        <span className="text-(--md-alert-note)">68</span>;
      </div>
    </div>
  )
}

function TextSizeStepper({
  value,
  onDecrease,
  onIncrease,
  onReset,
}: {
  value: number
  onDecrease: () => void
  onIncrease: () => void
  onReset: () => void
}) {
  return (
    <div className="flex items-center gap-2">
      <fieldset
        aria-label="Text size"
        className="m-0 flex h-[30px] w-fit min-w-0 items-center rounded-[7px] border-0 bg-surface-well p-0.5"
      >
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="h-[26px] w-7 text-muted-foreground hover:text-foreground"
          aria-label="Decrease text size"
          disabled={value <= ZOOM_MIN}
          onClick={onDecrease}
        >
          <Minus className="size-3.5" aria-hidden />
        </Button>
        <output
          aria-live="polite"
          className="w-[52px] text-center text-[12.5px] font-medium tabular-nums"
        >
          {value}%
        </output>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="h-[26px] w-7 text-muted-foreground hover:text-foreground"
          aria-label="Increase text size"
          disabled={value >= ZOOM_MAX}
          onClick={onIncrease}
        >
          <Plus className="size-3.5" aria-hidden />
        </Button>
      </fieldset>
      {value !== 100 && (
        <Button
          type="button"
          variant="ghost"
          size="xs"
          className="text-muted-foreground hover:text-foreground"
          onClick={onReset}
        >
          Reset
        </Button>
      )}
    </div>
  )
}

type UpdateStatus =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'up-to-date' }
  | { kind: 'available'; version: string }
  | { kind: 'downloading'; percent: number }
  | { kind: 'ready' }
  | { kind: 'error' }

function describeUpdateStatus(status: UpdateStatus, autoUpdateEnabled: boolean): string {
  switch (status.kind) {
    case 'checking':
      return 'Checking…'
    case 'up-to-date':
      return 'Up to date'
    case 'available':
      return `Version ${status.version} available`
    case 'downloading':
      return `Downloading… ${Math.round(status.percent)}%`
    case 'ready':
      return 'Update ready to install'
    case 'error':
      return 'Couldn’t check for updates'
    case 'idle':
      return autoUpdateEnabled ? 'Checks automatically' : 'Automatic checks are off'
  }
}

function UpdatesRow() {
  const autoUpdateEnabled = useAppStore((s) => s.autoUpdateEnabled)
  const [version, setVersion] = useState<string | null>(null)
  const [status, setStatus] = useState<UpdateStatus>({ kind: 'idle' })

  useEffect(() => {
    let cancelled = false
    void window.api
      .getAppVersion()
      .then((v) => {
        if (!cancelled) setVersion(v)
      })
      .catch(() => {})
    const unsubs = [
      window.api.onUpdateUpToDate(() => setStatus({ kind: 'up-to-date' })),
      window.api.onUpdateAvailable((info) =>
        setStatus({ kind: 'available', version: info.version }),
      ),
      window.api.onUpdateDownloadProgress((p) =>
        setStatus({ kind: 'downloading', percent: p.percent }),
      ),
      window.api.onUpdateDownloaded(() => setStatus({ kind: 'ready' })),
      window.api.onUpdateError(() => setStatus({ kind: 'error' })),
    ]
    return () => {
      cancelled = true
      for (const unsub of unsubs) unsub()
    }
  }, [])

  const checkNow = () => {
    setStatus({ kind: 'checking' })
    void window.api.checkForUpdates({ manual: true }).catch(() => setStatus({ kind: 'error' }))
  }

  return (
    <SettingsRow label={version ? `Mdow ${version}` : 'Mdow'}>
      <div className="flex items-center gap-2.5">
        <output
          aria-live="polite"
          className={cn(
            'min-w-0 flex-1 truncate text-xs',
            status.kind === 'error' ? 'text-destructive' : 'text-muted-foreground',
          )}
        >
          {describeUpdateStatus(status, autoUpdateEnabled)}
        </output>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={status.kind === 'checking' || status.kind === 'downloading'}
          onClick={checkNow}
        >
          Check now
        </Button>
      </div>
    </SettingsRow>
  )
}
