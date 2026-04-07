export function MockMarkdown() {
  return (
    <div className="grid grid-cols-[140px_1fr] min-h-[320px]">
      {/* File tree */}
      <aside className="border-r border-border-subtle bg-surface px-3 py-4 text-xs">
        <div className="mb-2 font-semibold text-muted-foreground/80 uppercase tracking-wider text-[10px]">
          Project
        </div>
        <ul className="space-y-1">
          <li className="text-muted-foreground">📁 docs</li>
          <li className="ml-3 text-foreground font-medium">📄 readme.md</li>
          <li className="ml-3 text-muted-foreground">📄 install.md</li>
          <li className="ml-3 text-muted-foreground">📄 features.md</li>
          <li className="text-muted-foreground">📁 notes</li>
        </ul>
      </aside>
      {/* Rendered content */}
      <div className="px-6 py-5 text-sm leading-relaxed">
        <h2 className="text-lg font-bold tracking-tight mb-2">Getting started</h2>
        <p className="text-muted-foreground mb-3">
          A quick overview of how mdow renders your markdown beautifully.
        </p>
        <pre className="rounded-md bg-surface border border-border-subtle px-3 py-2 mb-3 text-[11px] font-mono overflow-hidden">
          <span className="text-[oklch(0.55_0.17_260)]">function</span>{' '}
          <span className="text-[oklch(0.62_0.16_55)]">render</span>(){' '}
          <span className="text-muted-foreground">{'{'}</span>
          {'\n  '}
          <span className="text-[oklch(0.55_0.17_260)]">return</span>{' '}
          <span className="text-[oklch(0.6_0.15_160)]">'hello'</span>
          {'\n'}
          <span className="text-muted-foreground">{'}'}</span>
        </pre>
        <div className="rounded-md border border-border-subtle bg-surface px-3 py-3">
          <div className="flex items-center justify-center gap-2 text-[10px] text-muted-foreground">
            <span className="rounded border border-border px-2 py-0.5">Idea</span>
            <span>→</span>
            <span className="rounded border border-border px-2 py-0.5">Write</span>
            <span>→</span>
            <span className="rounded border border-border px-2 py-0.5">Render</span>
          </div>
        </div>
      </div>
    </div>
  )
}
