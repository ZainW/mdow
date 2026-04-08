import { BrowserFrame } from '~/components/browser-frame'
import { FeatureRow } from '~/components/feature-row'
import { GradientSection } from '~/components/gradient-section'
import { Screenshot } from './screenshot'

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
          <div className="rounded-md border border-primary/40 bg-primary/10 px-3 py-2 text-foreground shadow-soft">
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
      <Screenshot
        name="reading-light"
        alt="Mdow in light mode"
        className="rounded-lg shadow-soft"
      />
      <Screenshot name="reading-dark" alt="Mdow in dark mode" className="rounded-lg shadow-soft" />
    </div>
  )
}

function DropFileMockup() {
  return (
    <Screenshot
      name="empty-light"
      alt="Mdow empty state showing the drop zone"
      className="rounded-xl shadow-soft-lg"
    />
  )
}

export function LandingFeatures() {
  return (
    <GradientSection innerClassName="space-y-24 md:space-y-32">
      <FeatureRow
        title="Just drop a file in"
        description="Open a file, open a folder, or drag and drop. No setup, no accounts, no configuration. Mdow gets out of your way and lets you read."
        align="left"
      >
        <DropFileMockup />
      </FeatureRow>
      <FeatureRow
        title="Editor-quality syntax highlighting"
        description="Powered by Shiki — the same engine VS Code uses. 30+ languages, themed for both light and dark mode out of the box."
        align="right"
      >
        <CodeMockup />
      </FeatureRow>
      <FeatureRow
        title="Mermaid diagrams, rendered inline"
        description="Flowcharts, sequence diagrams, state machines — write them in plain text and watch them render where you'd expect."
        align="left"
      >
        <MermaidMockup />
      </FeatureRow>
      <FeatureRow
        title="Light and dark, just right"
        description="Warm stone tones in light mode, pure neutrals in dark. Follows your system, switches instantly, never fights your eyes."
        align="right"
      >
        <ThemeMockup />
      </FeatureRow>
    </GradientSection>
  )
}
