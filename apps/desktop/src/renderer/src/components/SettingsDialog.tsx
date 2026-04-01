import { useAppStore } from '../store/app-store'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from './ui/dialog'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from './ui/select'
import { Slider } from './ui/slider'
import { Separator } from './ui/separator'

const CONTENT_FONTS = [
  { value: 'inter', label: 'Inter', family: "'Inter', system-ui, sans-serif" },
  { value: 'system-sans', label: 'System Sans', family: 'system-ui, -apple-system, sans-serif' },
  {
    value: 'helvetica-neue',
    label: 'Helvetica Neue',
    family: "'Helvetica Neue', Helvetica, Arial, sans-serif",
  },
  { value: 'georgia', label: 'Georgia', family: "Georgia, 'Times New Roman', serif" },
  { value: 'palatino', label: 'Palatino', family: "Palatino, 'Palatino Linotype', serif" },
  { value: 'charter', label: 'Charter', family: "Charter, 'Bitstream Charter', Georgia, serif" },
  {
    value: 'times-new-roman',
    label: 'Times New Roman',
    family: "'Times New Roman', Times, serif",
  },
  {
    value: 'bookerly',
    label: 'Bookerly',
    family: "Bookerly, 'Iowan Old Style', Palatino, Georgia, serif",
  },
] as const

const CODE_FONTS = [
  {
    value: 'geist-mono',
    label: 'Geist Mono',
    family: "'Geist Mono', ui-monospace, monospace",
  },
  { value: 'sf-mono', label: 'SF Mono', family: "'SF Mono', SFMono-Regular, monospace" },
  { value: 'menlo', label: 'Menlo', family: 'Menlo, Monaco, monospace' },
  { value: 'consolas', label: 'Consolas', family: "Consolas, 'Courier New', monospace" },
  { value: 'fira-code', label: 'Fira Code', family: "'Fira Code', monospace" },
  { value: 'jetbrains-mono', label: 'JetBrains Mono', family: "'JetBrains Mono', monospace" },
  { value: 'source-code-pro', label: 'Source Code Pro', family: "'Source Code Pro', monospace" },
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
  const setContentFont = useAppStore((s) => s.setContentFont)
  const setCodeFont = useAppStore((s) => s.setCodeFont)
  const setFontSize = useAppStore((s) => s.setFontSize)
  const setLineHeight = useAppStore((s) => s.setLineHeight)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>Customize how markdown content is displayed.</DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-5">
          {/* Content Font */}
          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Content Font</span>
              <Select value={contentFont} onValueChange={(v) => v && setContentFont(v)}>
                <SelectTrigger className="w-[160px]" size="sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {CONTENT_FONTS.map((font) => (
                      <SelectItem key={font.value} value={font.value}>
                        <span style={{ fontFamily: font.family }}>{font.label}</span>
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>
            <p className="text-xs text-muted-foreground">
              The typeface used for paragraphs, headings, and lists.
            </p>
          </div>

          <Separator />

          {/* Code Font */}
          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Code Font</span>
              <Select value={codeFont} onValueChange={(v) => v && setCodeFont(v)}>
                <SelectTrigger className="w-[160px]" size="sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {CODE_FONTS.map((font) => (
                      <SelectItem key={font.value} value={font.value}>
                        <span style={{ fontFamily: font.family }}>{font.label}</span>
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>
            <p className="text-xs text-muted-foreground">
              The typeface used for inline code and code blocks.
            </p>
          </div>

          <Separator />

          {/* Font Size */}
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Font Size</span>
              <span className="text-xs tabular-nums text-muted-foreground">{fontSize}px</span>
            </div>
            <Slider
              value={[fontSize]}
              onValueChange={(v) => setFontSize(Number(v[0]))}
              min={13}
              max={24}
              step={1}
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>Small</span>
              <span>Large</span>
            </div>
          </div>

          <Separator />

          {/* Line Height */}
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Line Height</span>
              <span className="text-xs tabular-nums text-muted-foreground">
                {lineHeight.toFixed(1)}
              </span>
            </div>
            <Slider
              value={[lineHeight]}
              onValueChange={(v) => setLineHeight(Number(v[0]))}
              min={1.2}
              max={2.2}
              step={0.1}
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>Compact</span>
              <span>Spacious</span>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
