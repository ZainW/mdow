const FACTS = [
  {
    figure: '0',
    unit: 'browser engines',
    body: 'No Electron and no web view behind the page. Rune, a Rust UI framework with its own GPU renderer, draws every glyph, table and diagram.',
  },
  {
    figure: '1',
    unit: 'row per block',
    body: 'Each top-level block is its own virtual row, so a book-length file opens and scrolls as easily as a short one.',
  },
  {
    figure: '2',
    unit: 'systems, one app',
    body: 'The same reader on macOS and Linux, with ⌘ on the Mac, Ctrl everywhere else, and each system’s own file manager and editor.',
  },
  {
    figure: '4',
    unit: 'agents built in',
    body: 'Claude Code, Codex, Cursor and OpenCode, on the plans you already pay for. Mdow has no account of its own.',
  },
]

export function LandingEngine() {
  return (
    <section className="engine-band relative isolate overflow-hidden py-20 md:py-28">
      <div aria-hidden className="engine-grid absolute inset-0 -z-10" />
      <div className="mx-auto max-w-6xl px-5 sm:px-6">
        <div className="max-w-2xl">
          <p className="eyebrow !text-[var(--engine-accent)]">Under the hood</p>
          <h2 className="font-display mt-3 text-balance text-4xl leading-[1.08] sm:text-5xl">
            Not a web page in a window.
          </h2>
          <p className="mt-5 max-w-xl text-lg leading-relaxed text-[var(--engine-muted)]">
            Most markdown apps are browsers in disguise. Mdow is native code, start to finish, so it
            opens fast, stays light and scrolls smoothly.
          </p>
        </div>
        <dl className="mt-14 grid gap-px overflow-hidden rounded-2xl bg-[var(--engine-line)] sm:grid-cols-2 lg:grid-cols-4">
          {FACTS.map((f) => (
            <div key={f.unit} className="flex flex-col bg-[var(--engine-bg)] p-6 sm:p-7">
              <dt className="flex items-baseline gap-2.5">
                <span className="font-display text-[4rem] leading-none tracking-tight tabular-nums text-[var(--engine-accent)]">
                  {f.figure}
                </span>
                <span className="text-[15px] font-medium">{f.unit}</span>
              </dt>
              <dd className="mt-4 text-[15px] leading-relaxed text-[var(--engine-muted)]">
                {f.body}
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </section>
  )
}
