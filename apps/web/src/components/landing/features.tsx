import { BrowserFrame } from '~/components/browser-frame'
import { FeatureRow } from '~/components/feature-row'
import { GradientSection } from '~/components/gradient-section'

function CodeMockup() {
  return (
    <BrowserFrame title="example.ts">
      <div className="px-5 py-4 font-mono text-xs leading-relaxed">
        <div>
          <span className="text-[oklch(0.55_0.17_260)]">import</span>{' '}
          <span className="text-foreground">{'{ render }'}</span>{' '}
          <span className="text-[oklch(0.55_0.17_260)]">from</span>{' '}
          <span className="text-[oklch(0.6_0.15_160)]">'mdow'</span>
        </div>
        <div className="mt-2">
          <span className="text-[oklch(0.55_0.17_260)]">const</span>{' '}
          <span className="text-foreground">html</span>{' '}
          <span className="text-muted-foreground">=</span>{' '}
          <span className="text-[oklch(0.62_0.16_55)]">render</span>(
          <span className="text-[oklch(0.6_0.15_160)]">'# Hello'</span>)
        </div>
        <div className="mt-2 text-muted-foreground">// Beautifully highlighted</div>
      </div>
    </BrowserFrame>
  )
}

function MermaidMockup() {
  return (
    <BrowserFrame title="diagram.md">
      <div className="px-6 py-6">
        <div className="flex items-center justify-center gap-3 text-xs">
          <div className="rounded-md border border-border bg-surface px-3 py-2 shadow-soft">
            Idea
          </div>
          <div className="text-muted-foreground">→</div>
          <div className="rounded-md border border-border bg-surface px-3 py-2 shadow-soft">
            Draft
          </div>
          <div className="text-muted-foreground">→</div>
          <div className="rounded-md border border-primary/40 bg-primary/10 px-3 py-2 shadow-soft text-foreground">
            Publish
          </div>
        </div>
      </div>
    </BrowserFrame>
  )
}

function ThemeMockup() {
  return (
    <div className="grid grid-cols-2 gap-3">
      <BrowserFrame className="bg-[oklch(0.98_0.005_70)]">
        <div className="px-4 py-4 text-xs">
          <div className="font-semibold text-[oklch(0.13_0.02_50)]">Light</div>
          <div className="text-[oklch(0.45_0.015_50)] mt-1">Warm stone tones.</div>
        </div>
      </BrowserFrame>
      <BrowserFrame className="bg-[oklch(0.14_0_0)]">
        <div className="px-4 py-4 text-xs">
          <div className="font-semibold text-[oklch(0.92_0_0)]">Dark</div>
          <div className="text-[oklch(0.65_0_0)] mt-1">Pure neutrals.</div>
        </div>
      </BrowserFrame>
    </div>
  )
}

function FileTreeMockup() {
  return (
    <BrowserFrame title="Project">
      <div className="px-5 py-4 text-xs leading-loose">
        <div className="font-medium">📁 my-notes</div>
        <div className="ml-4 text-muted-foreground">📁 daily</div>
        <div className="ml-8 text-muted-foreground">📄 monday.md</div>
        <div className="ml-8 text-foreground font-medium">📄 tuesday.md</div>
        <div className="ml-4 text-muted-foreground">📁 projects</div>
        <div className="ml-8 text-muted-foreground">📄 readme.md</div>
        <div className="ml-4 text-muted-foreground">📄 todo.md</div>
      </div>
    </BrowserFrame>
  )
}

export function LandingFeatures() {
  return (
    <GradientSection innerClassName="space-y-24 md:space-y-32">
      <FeatureRow
        title="Editor-quality syntax highlighting"
        description="Powered by Shiki with the same engine VS Code uses. 30+ languages, themed for both light and dark mode out of the box."
        align="left"
      >
        <CodeMockup />
      </FeatureRow>
      <FeatureRow
        title="Mermaid diagrams, rendered inline"
        description="Flowcharts, sequence diagrams, state machines — write them in plain text and watch them render where you'd expect."
        align="right"
      >
        <MermaidMockup />
      </FeatureRow>
      <FeatureRow
        title="Light and dark, just right"
        description="Warm stone tones in light mode, pure neutrals in dark. Follows your system, switches instantly, never fights your eyes."
        align="left"
      >
        <ThemeMockup />
      </FeatureRow>
      <FeatureRow
        title="A file tree for the way you actually write"
        description="Open any folder and browse your markdown like a familiar workspace. Collapsible, keyboard-friendly, and out of your way."
        align="right"
      >
        <FileTreeMockup />
      </FeatureRow>
    </GradientSection>
  )
}
