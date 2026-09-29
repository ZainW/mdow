import { Link } from '@tanstack/react-router'
import { AppleIcon, LinuxIcon } from './icons'
import { PlatformDownloadRow } from './platform-download-row'

export function NativeDownloadSection({
  macUrl,
  linuxUrl,
}: {
  macUrl: string
  linuxUrl?: string | null
}) {
  return (
    <section className="mt-16" aria-labelledby="native-heading">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div className="max-w-xl">
          <div className="flex items-center gap-2.5">
            <h2 id="native-heading" className="text-xl font-semibold tracking-tight">
              Mdow Native
            </h2>
            <span className="rounded-full border border-border px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
              Beta
            </span>
          </div>
          <p className="mt-2 text-[15px] leading-relaxed text-muted-foreground">
            A GPU-rendered beta for Apple Silicon Macs and x64 Linux, built with gpuix. Runs
            alongside the regular Mdow app.
          </p>
        </div>
        <Link
          to="/docs/$"
          params={{ _splat: 'installation' }}
          hash="mdow-native-beta"
          className="link-underline shrink-0 text-sm text-muted-foreground"
        >
          Install notes
        </Link>
      </div>
      <div className="surface-card mt-6 divide-y divide-border-subtle overflow-hidden rounded-2xl">
        <PlatformDownloadRow
          icon={<AppleIcon className="size-5" />}
          platform="macOS"
          description="Apple Silicon · macOS 14 or newer"
          formats={[{ label: 'Mdow Native', detail: '.zip', url: macUrl }]}
        />
        {linuxUrl ? (
          <PlatformDownloadRow
            icon={<LinuxIcon className="size-5" />}
            platform="Linux"
            description="x64 · make the AppImage executable and run it"
            formats={[{ label: 'Mdow Native', detail: '.AppImage', url: linuxUrl }]}
          />
        ) : null}
      </div>
    </section>
  )
}
