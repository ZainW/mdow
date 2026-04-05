import { Link } from '@tanstack/react-router'

export function Hero() {
  return (
    <section className="mx-auto max-w-4xl px-6 py-24 text-center md:py-32">
      <h1 className="text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl">
        A quiet place to read markdown
      </h1>
      <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
        Beautiful, fast, and distraction-free. Mdow renders your markdown with syntax highlighting,
        Mermaid diagrams, and a reading experience that gets out of the way.
      </p>
      <div className="mt-10 flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
        <Link
          to="/download"
          className="inline-flex h-11 items-center rounded-lg bg-primary px-8 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
        >
          Download for free
        </Link>
        <Link
          to="/docs"
          className="inline-flex h-11 items-center rounded-lg border px-8 text-sm font-medium transition-colors hover:bg-muted"
        >
          Read the docs
        </Link>
      </div>
    </section>
  )
}
