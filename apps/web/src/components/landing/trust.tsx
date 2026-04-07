import { GradientSection } from '~/components/gradient-section'

const formats = ['.md', '.markdown', '.mdx']
const platforms = [
  { name: 'macOS', icon: '🍎' },
  { name: 'Windows', icon: '🪟' },
  { name: 'Linux', icon: '🐧' },
]

export function LandingTrust() {
  return (
    <GradientSection innerClassName="text-center py-16 md:py-20">
      <p className="text-sm font-medium uppercase tracking-wider text-muted-foreground">
        Free to download
      </p>
      <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
        {formats.map((f) => (
          <span
            key={f}
            className="rounded-full border border-border bg-card px-4 py-1.5 font-mono text-xs text-muted-foreground"
          >
            {f}
          </span>
        ))}
      </div>
      <div className="mt-6 flex flex-wrap items-center justify-center gap-6 text-sm text-muted-foreground">
        {platforms.map((p) => (
          <div key={p.name} className="flex items-center gap-2">
            <span className="text-base" aria-hidden>
              {p.icon}
            </span>
            <span>{p.name}</span>
          </div>
        ))}
      </div>
    </GradientSection>
  )
}
