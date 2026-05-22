import {
  AlertCircle,
  ArrowLeftRight,
  Check,
  ChevronRight,
  Clock,
  Copy,
  Download,
  File,
  FileText,
  FileX,
  Folder,
  FolderOpen,
  List,
  Minus,
  Monitor,
  Moon,
  Plus,
  RotateCcw,
  Search,
  Settings,
  ShieldAlert,
  FoldHorizontal,
  Sun,
  Trash2,
  X,
  type LucideIcon,
} from 'lucide-react'
import { iconStroke } from '../lib/icons'

const SIZES = [
  { label: '10px', className: 'size-2.5', px: 10 },
  { label: '12px', className: 'size-3', px: 12 },
  { label: '14px', className: 'size-3.5', px: 14 },
  { label: '16px', className: 'size-4', px: 16 },
] as const

const STROKES = [1.5, iconStroke.default, iconStroke.emphasis] as const

const SAMPLE_ICONS: { label: string; Icon: LucideIcon }[] = [
  { label: 'Clock', Icon: Clock },
  { label: 'Folder', Icon: Folder },
  { label: 'List', Icon: List },
  { label: 'Search', Icon: Search },
  { label: 'File', Icon: File },
  { label: 'FolderOpen', Icon: FolderOpen },
  { label: 'Settings', Icon: Settings },
  { label: 'FileText', Icon: FileText },
  { label: 'X', Icon: X },
  { label: 'ChevronRight', Icon: ChevronRight },
  { label: 'ArrowLeftRight', Icon: ArrowLeftRight },
  { label: 'FoldHorizontal', Icon: FoldHorizontal },
  { label: 'Sun', Icon: Sun },
  { label: 'Moon', Icon: Moon },
  { label: 'Monitor', Icon: Monitor },
  { label: 'Minus', Icon: Minus },
  { label: 'Plus', Icon: Plus },
  { label: 'RotateCcw', Icon: RotateCcw },
  { label: 'Download', Icon: Download },
  { label: 'Check', Icon: Check },
  { label: 'Copy', Icon: Copy },
  { label: 'FileX', Icon: FileX },
  { label: 'Trash2', Icon: Trash2 },
  { label: 'ShieldAlert', Icon: ShieldAlert },
  { label: 'AlertCircle', Icon: AlertCircle },
]

function setTheme(mode: 'light' | 'dark') {
  document.documentElement.classList.toggle('dark', mode === 'dark')
}

function IconCell({
  Icon,
  sizeClass,
  strokeWidth,
  muted,
}: {
  Icon: LucideIcon
  sizeClass: string
  strokeWidth: number
  muted: boolean
}) {
  return (
    <div
      className={`flex h-10 w-10 items-center justify-center rounded-md border border-border-subtle ${
        muted ? 'text-muted-foreground' : 'text-foreground'
      }`}
    >
      <Icon className={sizeClass} strokeWidth={strokeWidth} aria-hidden />
    </div>
  )
}

export function IconLab() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <header className="flex shrink-0 items-center gap-3 border-b border-border-subtle px-4 py-2">
        <h1 className="text-sm font-medium">Icon Lab</h1>
        <span className="text-xs text-muted-foreground">Lucide size × stroke matrix</span>
        <div className="ml-auto flex gap-1">
          <button
            type="button"
            className="rounded-md px-2 py-1 text-xs hover:bg-muted"
            onClick={() => setTheme('light')}
          >
            Light
          </button>
          <button
            type="button"
            className="rounded-md px-2 py-1 text-xs hover:bg-muted"
            onClick={() => setTheme('dark')}
          >
            Dark
          </button>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        <section className="mb-8">
          <h2 className="mb-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
            Settings theme — outline vs filled (14px, stroke 2)
          </h2>
          <div className="flex gap-4">
            {(
              [
                { label: 'System', Icon: Monitor },
                { label: 'Light', Icon: Sun },
                { label: 'Dark', Icon: Moon },
              ] as const
            ).map(({ label, Icon }) => (
              <div key={label} className="flex flex-col items-center gap-2">
                <div className="flex gap-2">
                  <div className="flex flex-col items-center gap-1 rounded-md bg-muted p-2">
                    <Icon className="size-3.5 text-muted-foreground" strokeWidth={2} fill="none" />
                    <span className="text-[10px] text-muted-foreground">idle</span>
                  </div>
                  <div className="flex flex-col items-center gap-1 rounded-md bg-background p-2 shadow-sm ring-1 ring-foreground/10">
                    <Icon
                      className="size-3.5 text-foreground"
                      strokeWidth={2}
                      fill="currentColor"
                    />
                    <span className="text-[10px] text-muted-foreground">active</span>
                  </div>
                </div>
                <span className="text-[10px] text-muted-foreground">{label}</span>
              </div>
            ))}
          </div>
        </section>

        {(['muted', 'foreground'] as const).map((tone) => (
          <section key={tone} className="mb-10">
            <h2 className="mb-3 text-xs font-medium uppercase tracking-wider text-muted-foreground">
              text-{tone === 'muted' ? 'muted-foreground' : 'foreground'}
            </h2>
            {STROKES.map((stroke) => (
              <div key={stroke} className="mb-6">
                <h3 className="mb-2 text-[11px] text-muted-foreground">strokeWidth {stroke}</h3>
                <div className="overflow-x-auto">
                  <table className="border-collapse text-[10px]">
                    <thead>
                      <tr>
                        <th className="sticky left-0 bg-background p-1 text-left font-normal text-muted-foreground">
                          icon
                        </th>
                        {SIZES.map((s) => (
                          <th key={s.px} className="p-1 font-normal text-muted-foreground">
                            {s.label}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {SAMPLE_ICONS.map(({ label, Icon }) => (
                        <tr key={label}>
                          <td className="sticky left-0 bg-background p-1 text-muted-foreground">
                            {label}
                          </td>
                          {SIZES.map((s) => (
                            <td key={s.px} className="p-0.5">
                              <IconCell
                                Icon={Icon}
                                sizeClass={s.className}
                                strokeWidth={stroke}
                                muted={tone === 'muted'}
                              />
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ))}
          </section>
        ))}
      </div>
    </div>
  )
}
