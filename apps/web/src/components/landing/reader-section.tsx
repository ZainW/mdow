import { AppWindow } from './app-window'

const PRINCIPLES = [
  {
    title: 'Read-only by design',
    body: 'Mdow never changes your files, and neither does the AI companion. Your editor stays the source of truth.',
  },
  {
    title: 'No setup, no account',
    body: 'Open a file, open a folder, or drag something in. The companion uses providers already configured on your machine.',
  },
  {
    title: 'Plain files, anywhere',
    body: 'Markdown, MDX, and local HTML straight from disk. Nothing to import, nothing to sync.',
  },
]

const FORMATS = ['.md', '.markdown', '.mdx', '.html']

export function LandingReaderSection() {
  return (
    <section className="border-y border-border-subtle bg-surface/70 py-20 md:py-28 dark:bg-surface/40">
      <div className="mx-auto grid max-w-6xl items-center gap-14 px-5 sm:px-6 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-16">
        <div>
          <p className="eyebrow">Why a reader</p>
          <h2 className="font-display mt-3 text-4xl leading-[1.08] sm:text-5xl">
            Write in your editor.
            <br />
            Read in Mdow.
          </h2>
          <p className="mt-5 text-lg leading-relaxed text-muted-foreground">
            Editors are built for changing text. Reading deserves its own calm surface, with real
            typography, working links, and an outline that keeps its place.
          </p>
          <dl className="mt-10 space-y-6">
            {PRINCIPLES.map((p) => (
              <div key={p.title} className="border-l-2 border-accent/40 pl-4">
                <dt className="font-medium">{p.title}</dt>
                <dd className="mt-1 text-[15px] leading-relaxed text-muted-foreground">{p.body}</dd>
              </div>
            ))}
          </dl>
          <ul className="mt-10 flex flex-wrap gap-2" aria-label="Supported file types">
            {FORMATS.map((f) => (
              <li
                key={f}
                className="rounded-md border border-border bg-card px-2.5 py-1 font-mono text-xs text-muted-foreground"
              >
                {f}
              </li>
            ))}
          </ul>
        </div>
        <AppWindow
          name="outline"
          alt="Mdow's outline sidebar highlighting the current section of a long document, with a table, a blockquote, and links"
        />
      </div>
    </section>
  )
}
