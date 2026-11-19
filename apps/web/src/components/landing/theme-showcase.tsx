import { useEffect, useRef, useState, type CSSProperties } from 'react'
import { cn } from '~/lib/utils'

interface Theme {
  id: string
  name: string
  /** Omarchy's name for it; Mdow's own looks have none. */
  omarchy?: string
  bg: string
  side: string
  fg: string
  muted: string
  line: string
  accent: string
  keyword: string
  string: string
  fn: string
}

// Omarchy's colours are copied from each theme's colors.toml, the same file Mdow reads.
const THEMES: Theme[] = [
  {
    id: 'paper',
    name: 'Mdow Light',
    bg: '#fbf8f5',
    side: '#f3efea',
    fg: '#232220',
    muted: '#77716a',
    line: '#e4ded6',
    accent: '#c4643b',
    keyword: '#a3368f',
    string: '#3c7a4a',
    fn: '#2f62a8',
  },
  {
    id: 'ink',
    name: 'Mdow Dark',
    bg: '#1c1b1a',
    side: '#161514',
    fg: '#e7e3de',
    muted: '#8f8982',
    line: '#2f2d2b',
    accent: '#e08156',
    keyword: '#d98cc8',
    string: '#8fc29a',
    fn: '#8fb3e8',
  },
  {
    id: 'tokyo-night',
    name: 'Tokyo Night',
    omarchy: 'tokyo-night',
    bg: '#1a1b26',
    side: '#13141c',
    fg: '#a9b1d6',
    muted: '#565f89',
    line: '#292e42',
    accent: '#7aa2f7',
    keyword: '#ad8ee6',
    string: '#9ece6a',
    fn: '#7aa2f7',
  },
  {
    id: 'catppuccin',
    name: 'Catppuccin',
    omarchy: 'catppuccin',
    bg: '#1e1e2e',
    side: '#161622',
    fg: '#cdd6f4',
    muted: '#6c7086',
    line: '#45475a',
    accent: '#89b4fa',
    keyword: '#f5c2e7',
    string: '#a6e3a1',
    fn: '#89b4fa',
  },
  {
    id: 'gruvbox',
    name: 'Gruvbox',
    omarchy: 'gruvbox',
    bg: '#282828',
    side: '#1e1e1e',
    fg: '#d4be98',
    muted: '#7c6f64',
    line: '#504945',
    accent: '#7daea3',
    keyword: '#d3869b',
    string: '#a9b665',
    fn: '#d8a657',
  },
  {
    id: 'rose-pine',
    name: 'Rosé Pine',
    omarchy: 'rose-pine',
    bg: '#faf4ed',
    side: '#ede7e1',
    fg: '#575279',
    muted: '#9893a5',
    line: '#dfdad9',
    accent: '#56949f',
    keyword: '#907aa9',
    string: '#286983',
    fn: '#b4637a',
  },
  {
    id: 'everforest',
    name: 'Everforest',
    omarchy: 'everforest',
    bg: '#2d353b',
    side: '#21272c',
    fg: '#d3c6aa',
    muted: '#859289',
    line: '#3d484d',
    accent: '#7fbbb3',
    keyword: '#d699b6',
    string: '#a7c080',
    fn: '#dbbc7f',
  },
  {
    id: 'nord',
    name: 'Nord',
    omarchy: 'nord',
    bg: '#2e3440',
    side: '#222730',
    fg: '#d8dee9',
    muted: '#7b8597',
    line: '#434c5e',
    accent: '#88c0d0',
    keyword: '#b48ead',
    string: '#a3be8c',
    fn: '#81a1c1',
  },
]

const CYCLE_MS = 2800

function vars(t: Theme): CSSProperties {
  return {
    '--t-bg': t.bg,
    '--t-side': t.side,
    '--t-fg': t.fg,
    '--t-muted': t.muted,
    '--t-line': t.line,
    '--t-accent': t.accent,
    '--t-keyword': t.keyword,
    '--t-string': t.string,
    '--t-fn': t.fn,
  } as CSSProperties
}

export function LandingThemeShowcase() {
  const [index, setIndex] = useState(2)
  const [auto, setAuto] = useState(true)
  const ref = useRef<HTMLDivElement>(null)
  const theme = THEMES[index]

  // Cycle through the themes while the section is on screen, until someone picks one.
  useEffect(() => {
    if (!auto || !ref.current) return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
    let timer: number | undefined
    const observer = new IntersectionObserver(([entry]) => {
      window.clearInterval(timer)
      if (entry.isIntersecting) {
        timer = window.setInterval(() => setIndex((i) => (i + 1) % THEMES.length), CYCLE_MS)
      }
    })
    observer.observe(ref.current)
    return () => {
      observer.disconnect()
      window.clearInterval(timer)
    }
  }, [auto])

  const command = theme.omarchy
    ? `omarchy theme set ${theme.omarchy}`
    : `# Settings › Appearance › ${theme.id === 'paper' ? 'Light' : 'Dark'}`

  return (
    <section id="themes" className="scroll-mt-20 overflow-x-clip py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-5 sm:px-6">
        <div className="grid gap-12 lg:grid-cols-[minmax(0,4fr)_minmax(0,7fr)] lg:items-center lg:gap-16">
          <div>
            <p className="eyebrow">Made for Omarchy</p>
            <h2 className="font-display mt-3 text-balance text-4xl leading-[1.08] sm:text-5xl">
              It wears your theme.
            </h2>
            <p className="mt-5 text-lg leading-relaxed text-muted-foreground">
              On Omarchy, Mdow takes its colours from the theme you’re running and its corners from
              Hyprland. Run{' '}
              <code className="font-mono text-[0.88em] text-foreground">omarchy theme set</code> and
              every open window changes colour on the spot, diagrams and formulas included.
              Elsewhere it has its own warm paper and soft ink.
            </p>

            <div
              role="radiogroup"
              aria-label="Preview a theme"
              className="mt-8 flex flex-wrap gap-2"
            >
              {THEMES.map((t, i) => (
                <button
                  key={t.id}
                  type="button"
                  role="radio"
                  aria-checked={i === index}
                  onClick={() => {
                    setAuto(false)
                    setIndex(i)
                  }}
                  className={cn(
                    'theme-chip flex items-center gap-2 rounded-full border py-1.5 pl-1.5 pr-3 text-[13px] transition-[border-color,background-color,color] duration-150 ease',
                    i === index
                      ? 'border-foreground/30 bg-card text-foreground'
                      : 'border-border-subtle text-muted-foreground hover:border-border hover:text-foreground',
                  )}
                >
                  <span
                    aria-hidden
                    className="flex size-5 items-center justify-center rounded-full ring-1 ring-inset ring-black/10"
                    style={{ background: t.bg }}
                  >
                    <span className="size-2 rounded-full" style={{ background: t.accent }} />
                  </span>
                  {t.name}
                </button>
              ))}
            </div>
          </div>

          <div ref={ref} className="relative">
            <div
              aria-hidden
              className="theme-halo pointer-events-none absolute -inset-10 -z-10 rounded-[3rem]"
              style={vars(theme)}
            />
            <ThemedWindow theme={theme} />
            <div className="mt-4 flex items-center gap-2.5 rounded-xl border border-border-subtle bg-card px-4 py-3 font-mono text-[12.5px]">
              <span className="text-accent" aria-hidden>
                ❯
              </span>
              <span key={command} className="theme-command truncate text-foreground">
                {command}
              </span>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}

function ThemedWindow({ theme }: { theme: Theme }) {
  return (
    <div
      role="img"
      aria-label={`Mdow in the ${theme.name} theme, showing a document with a code block and a diagram`}
      className="themed overflow-hidden rounded-2xl shadow-[0_30px_80px_-30px_hsl(var(--shadow-color)/0.45),0_0_0_1px_color-mix(in_oklch,var(--foreground)_10%,transparent)]"
      style={vars(theme)}
    >
      <div aria-hidden className="flex text-[12px]">
        <aside className="themed-side hidden w-44 shrink-0 flex-col gap-0.5 border-r px-2 py-3 sm:flex">
          <p className="themed-muted px-2 pb-1.5 text-[10.5px] font-medium uppercase tracking-wider">
            Outline
          </p>
          {['How Lantern syncs', 'The merge loop', 'Ordering', 'Checklist'].map((h, i) => (
            <span
              key={h}
              className={cn(
                'rounded-md px-2 py-1',
                i === 1 ? 'themed-active font-medium' : 'themed-muted',
                i > 0 && 'pl-4',
              )}
            >
              {h}
            </span>
          ))}
        </aside>
        <div className="min-w-0 flex-1">
          <div className="themed-side flex items-center gap-1 border-b px-3 py-2">
            <span className="themed-tab rounded-md px-2.5 py-1 font-medium">sync.md</span>
            <span className="themed-muted px-2.5 py-1">README.md</span>
          </div>
          <div className="space-y-3.5 px-6 py-5 sm:px-8 sm:py-6">
            <p className="font-serif text-[22px] font-semibold leading-tight">The merge loop</p>
            <p className="leading-relaxed opacity-90">
              Each edit carries a <strong>vector clock</strong>, so two devices can work offline and
              still agree on what happened when they meet again.
            </p>
            <div className="themed-callout rounded-r-md border-l-2 px-3 py-2 leading-relaxed">
              <span className="themed-accent font-medium">Note</span> Nothing is discarded silently.
            </div>
            <pre className="themed-code overflow-hidden rounded-lg border px-3.5 py-3 font-mono text-[11.5px] leading-[1.7]">
              <span className="themed-kw">export function</span>{' '}
              <span className="themed-fn">merge</span>(a, b) {'{'}
              {'\n  '}
              <span className="themed-kw">if</span> (a.clock {'>'} b.clock){' '}
              <span className="themed-kw">return</span> a{'\n  '}
              <span className="themed-kw">return</span> {'{ ...b, tag: '}
              <span className="themed-str">{"'sibling'"}</span>
              {' }'}
              {'\n}'}
            </pre>
            <svg viewBox="0 0 300 44" className="h-auto w-full max-w-sm" fill="none">
              {['Edit', 'Save', 'Watch', 'Merge'].map((label, i) => (
                <g key={label}>
                  {i > 0 && (
                    <path d={`M${i * 76 - 8} 22h12`} className="themed-stroke" strokeWidth="1.25" />
                  )}
                  <rect
                    x={i * 76 + 4}
                    y={6}
                    width={60}
                    height={32}
                    rx={7}
                    className={i === 3 ? 'themed-node-accent' : 'themed-node'}
                    strokeWidth="1.25"
                  />
                  <text
                    x={i * 76 + 34}
                    y={26}
                    textAnchor="middle"
                    fontSize="11"
                    fontWeight={500}
                    className={cn('font-sans', i === 3 ? 'themed-fill-accent' : 'themed-fill')}
                  >
                    {label}
                  </text>
                </g>
              ))}
            </svg>
          </div>
        </div>
      </div>
    </div>
  )
}
