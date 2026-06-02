import { BrowserFrame } from '~/components/browser-frame'
import { FeatureRow } from '~/components/feature-row'
import { GradientSection } from '~/components/gradient-section'
import { Screenshot } from './screenshot'

function CodeMockup() {
  return (
    <BrowserFrame
      title="example.ts"
      ariaLabel="Illustration of TypeScript syntax highlighting in a code block"
    >
      <div className="p-5 font-mono text-[13px] leading-[1.7]">
        <div>
          <span className="text-primary">import</span>
          <span className="text-foreground">{' { render } '}</span>
          <span className="text-primary">from</span>
          <span className="text-[oklch(0.65_0.14_160)]"> 'mdow'</span>
        </div>
        <div className="mt-1.5">
          <span className="text-primary">const</span>
          <span className="text-foreground"> html </span>
          <span className="text-muted-foreground">= </span>
          <span className="text-[oklch(0.6_0.13_242)]">render</span>
          <span className="text-muted-foreground">(</span>
          <span className="text-[oklch(0.65_0.14_160)]">'# Hello'</span>
          <span className="text-muted-foreground">)</span>
        </div>
        <div className="mt-3 text-muted-foreground/70">
          <span className="text-muted-foreground/50">{'// '}</span>
          Editor-quality highlighting via Shiki
        </div>
      </div>
    </BrowserFrame>
  )
}

function MermaidMockup() {
  return (
    <BrowserFrame
      title="diagram.md"
      ariaLabel="Illustration of an inline Mermaid flowchart with Idea, Draft, and Publish steps"
    >
      <div className="px-6 py-8">
        <div className="flex items-center justify-center gap-4 text-sm">
          <div className="rounded-md border border-border-subtle bg-surface px-4 py-2.5 shadow-soft">
            Idea
          </div>
          <svg
            width="24"
            height="12"
            viewBox="0 0 24 12"
            fill="none"
            className="text-muted-foreground/60"
            aria-hidden
          >
            <path d="M0 6h20m0 0l-4-4m4 4l-4 4" stroke="currentColor" strokeWidth="1.5" />
          </svg>
          <div className="rounded-md border border-border-subtle bg-surface px-4 py-2.5 shadow-soft">
            Draft
          </div>
          <svg
            width="24"
            height="12"
            viewBox="0 0 24 12"
            fill="none"
            className="text-muted-foreground/60"
            aria-hidden
          >
            <path d="M0 6h20m0 0l-4-4m4 4l-4 4" stroke="currentColor" strokeWidth="1.5" />
          </svg>
          <div className="rounded-md border border-primary/25 bg-primary/8 px-4 py-2.5 font-medium text-foreground shadow-soft ring-1 ring-primary/10">
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
        className="rounded-lg shadow-card"
      />
      <Screenshot name="reading-dark" alt="Mdow in dark mode" className="rounded-lg shadow-card" />
    </div>
  )
}

function DropFileMockup() {
  return (
    <Screenshot
      name="empty-light"
      alt="Mdow welcome screen with open file and open folder actions"
      className="rounded-lg shadow-elevated"
    />
  )
}

function SidebarMockup() {
  return (
    <Screenshot
      name="sidebar-light"
      alt="Mdow folder sidebar with a file tree and tabbed documents"
      className="rounded-lg shadow-elevated"
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
        title="Browse folders like a project"
        description="Open a directory and navigate your markdown files in a collapsible tree. Switch between Recents, Outline, and Folder views from the sidebar."
        align="right"
      >
        <SidebarMockup />
      </FeatureRow>
      <FeatureRow
        title="Editor-quality syntax highlighting"
        description="Powered by Shiki — the same engine VS Code uses. 30+ languages, themed for both light and dark mode out of the box."
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
        description="Clean white in light mode, cool blue-gray in dark. Follows your system, switches instantly, never fights your eyes."
        align="left"
      >
        <ThemeMockup />
      </FeatureRow>
    </GradientSection>
  )
}
