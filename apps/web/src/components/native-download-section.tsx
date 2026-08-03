import { DownloadCard } from '~/components/download-card'

export function NativeDownloadSection({ betaUrl }: { betaUrl: string }) {
  return (
    <section className="mt-10 border-t border-border-subtle pt-8">
      <div className="mx-auto mb-5 max-w-2xl text-center">
        <p className="text-xs font-semibold uppercase tracking-wide text-primary">Beta</p>
        <h2 className="mt-2 text-2xl font-semibold tracking-tight">Mdow Native</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          A GPUI beta for Apple Silicon Macs running macOS 14 or newer.
        </p>
        <p className="mt-1 text-sm text-muted-foreground">Runs alongside the regular Mdow app.</p>
      </div>
      <div className="mx-auto max-w-sm">
        <DownloadCard
          platform="Mdow Native — GPUI beta"
          icon={'\u{2318}'}
          formats={[{ label: 'Download Mdow Native (.zip)', url: betaUrl }]}
        />
      </div>
    </section>
  )
}
