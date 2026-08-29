import { DownloadCard } from '~/components/download-card'

export function NativeDownloadSection({
  macUrl,
  linuxUrl,
}: {
  macUrl: string
  linuxUrl?: string | null
}) {
  return (
    <section className="mt-10 border-t border-border-subtle pt-8">
      <div className="mx-auto mb-5 max-w-2xl text-center">
        <p className="text-xs font-semibold uppercase tracking-wide text-primary">Beta</p>
        <h2 className="mt-2 text-2xl font-semibold tracking-tight">Mdow Native</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          A GPUI beta for Apple Silicon Macs and x64 Linux.
        </p>
        <p className="mt-1 text-sm text-muted-foreground">Runs alongside the regular Mdow app.</p>
      </div>
      <div className={`mx-auto grid gap-6 ${linuxUrl ? 'max-w-2xl sm:grid-cols-2' : 'max-w-sm'}`}>
        <DownloadCard
          platform="macOS — Apple Silicon"
          icon={'\u{2318}'}
          formats={[{ label: 'Download Mdow Native (.zip)', url: macUrl }]}
        />
        {linuxUrl ? (
          <DownloadCard
            platform="Linux — x64"
            icon={'\u{1F427}'}
            formats={[{ label: 'Download Mdow Native (.zip)', url: linuxUrl }]}
          />
        ) : null}
      </div>
    </section>
  )
}
