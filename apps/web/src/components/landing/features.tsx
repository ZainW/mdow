import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { useModKey } from '~/hooks/use-mod-key'
import { cn } from '~/lib/utils'
import { ArrowRightIcon, CheckIcon } from '../icons'
import {
  CodeMock,
  CompanionMock,
  DiagramMock,
  LiveReloadMock,
  PaletteMock,
  SpeedMock,
  TreeMock,
} from './mocks'

const EXTRAS = [
  'Tabs and side-by-side split view',
  'Find in document with highlighted matches',
  'Local HTML files in a sandboxed viewer',
  'Light and dark themes that follow your system',
  'Drag and drop files or whole folders',
  'Wide reading mode for fewer distractions',
]

export function LandingFeatures() {
  const modKey = useModKey() || '⌘'

  return (
    <section id="features" className="scroll-mt-20 py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-5 sm:px-6">
        <div className="max-w-2xl">
          <p className="eyebrow">Features</p>
          <h2 className="font-display mt-3 text-4xl leading-[1.08] sm:text-5xl">
            Everything a reader needs. Nothing an editor does.
          </h2>
          <p className="mt-5 max-w-xl text-lg leading-relaxed text-muted-foreground">
            Keep writing in the editor you love. Mdow is where you go to read, navigate, and
            understand what you wrote.
          </p>
        </div>

        <div className="mt-14 grid gap-4 md:grid-cols-2 lg:grid-cols-6">
          <FeatureCard
            className="md:col-span-2 lg:col-span-4"
            title="Ask your documents"
            description={
              <>
                Chat about the open file, a whole folder, or anything you{' '}
                <code className="font-mono text-[0.92em] text-foreground">@</code>-mention. Answers
                come from the ACP agent you already run, like OpenCode or Codex, with citations that
                jump straight to the source.
              </>
            }
            link={{ label: 'How the companion works', slug: 'ai-companion' }}
          >
            <CompanionMock />
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-2"
            title="Fast with huge files"
            description="Multi-megabyte documents open in about a second. Highlighting and diagrams render as you scroll to them."
          >
            <SpeedMock />
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-2"
            title="Editor-grade code"
            description="Shiki highlighting, the engine behind VS Code, themed for light and dark."
          >
            <CodeMock />
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-2"
            title="Mermaid, inline"
            description="Flowcharts and sequence diagrams render right where you wrote them."
          >
            <DiagramMock />
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-2"
            title="Folders and outlines"
            description="Browse a project as a tree and jump between headings in long reads."
          >
            <TreeMock />
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-3"
            title="Keyboard first"
            description={`Jump to any file or action from the command palette with ${modKey}K. Hold ${modKey} for a cheat sheet.`}
            link={{ label: 'All shortcuts', slug: 'shortcuts' }}
          >
            <PaletteMock modKey={modKey} />
          </FeatureCard>
          <FeatureCard
            className="md:col-span-2 lg:col-span-3"
            title="Live as you save"
            description="Mdow watches your files. Save in your editor and only the parts that changed re-render, right where you left off."
          >
            <LiveReloadMock />
          </FeatureCard>
        </div>

        <ul className="mt-12 grid gap-x-8 gap-y-3 border-t border-border-subtle pt-10 text-[15px] sm:grid-cols-2 lg:grid-cols-3">
          {EXTRAS.map((item) => (
            <li key={item} className="flex items-start gap-3 text-muted-foreground">
              <CheckIcon className="mt-1 size-4 shrink-0 text-accent" />
              {item}
            </li>
          ))}
        </ul>
      </div>
    </section>
  )
}

function FeatureCard({
  title,
  description,
  link,
  className,
  children,
}: {
  title: string
  description: ReactNode
  link?: { label: string; slug: string }
  className?: string
  children: ReactNode
}) {
  return (
    <article className={cn('surface-card flex flex-col overflow-hidden rounded-2xl', className)}>
      <div className="flex flex-1 items-center bg-surface/60 px-6 py-8 sm:px-8 dark:bg-background/40">
        <div className="w-full">{children}</div>
      </div>
      <div className="border-t border-border-subtle px-6 py-5 sm:px-8">
        <h3 className="font-semibold tracking-tight">{title}</h3>
        <p className="mt-1.5 text-[15px] leading-relaxed text-muted-foreground">{description}</p>
        {link && (
          <Link
            to="/docs/$"
            params={{ _splat: link.slug }}
            className="group mt-3 inline-flex items-center gap-1.5 text-sm font-medium text-foreground"
          >
            {link.label}
            <ArrowRightIcon className="size-3.5 transition-transform duration-200 ease-out group-hover:translate-x-0.5" />
          </Link>
        )}
      </div>
    </article>
  )
}
