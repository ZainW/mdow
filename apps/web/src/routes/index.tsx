import { createFileRoute } from '@tanstack/react-router'
import { Hero } from '~/components/hero'

export const Route = createFileRoute('/')({
  component: HomePage,
})

const features = [
  {
    title: 'Syntax highlighting',
    description: 'Editor-quality highlighting for 30+ languages powered by Shiki.',
  },
  {
    title: 'Mermaid diagrams',
    description: 'Flowcharts, sequence diagrams, and more rendered inline.',
  },
  {
    title: 'Light & dark themes',
    description: 'Warm stone tones in light mode, pure neutrals in dark. Follows your system.',
  },
  {
    title: 'File tree sidebar',
    description: 'Open a folder and browse your markdown files in a collapsible tree view.',
  },
]

function HomePage() {
  return (
    <>
      <Hero />
      <section className="mx-auto max-w-5xl px-6 py-16">
        <div className="grid gap-8 sm:grid-cols-2">
          {features.map((f) => (
            <div key={f.title} className="rounded-lg border bg-card p-6">
              <h3 className="mb-2 font-semibold">{f.title}</h3>
              <p className="text-sm text-muted-foreground">{f.description}</p>
            </div>
          ))}
        </div>
      </section>
    </>
  )
}
