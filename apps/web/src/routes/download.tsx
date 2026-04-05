import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { DownloadCard } from '~/components/download-card'
import { seo } from '~/lib/seo'

const detectOS = createServerFn({ method: 'GET' }).handler(({ request }) => {
  const ua = request?.headers.get('user-agent') || ''
  if (ua.includes('Mac')) return 'mac'
  if (ua.includes('Windows')) return 'windows'
  if (ua.includes('Linux')) return 'linux'
  return 'mac'
})

export const Route = createFileRoute('/download')({
  loader: () => detectOS(),
  head: () => ({
    meta: seo({
      title: 'Download Mdow',
      description: 'Download Mdow for Mac, Windows, or Linux.',
    }),
  }),
  component: DownloadPage,
})

// Replace with actual Cloudflare R2 URLs once binaries are uploaded
const DOWNLOAD_BASE = '#'

const platforms = [
  {
    id: 'mac',
    platform: 'macOS',
    icon: '\u{1F4BB}',
    formats: [
      { label: 'Download .dmg', url: `${DOWNLOAD_BASE}/Mdow.dmg` },
      { label: 'Download .zip', url: `${DOWNLOAD_BASE}/Mdow-mac.zip` },
    ],
  },
  {
    id: 'windows',
    platform: 'Windows',
    icon: '\u{1FAA9}',
    formats: [{ label: 'Download Installer', url: `${DOWNLOAD_BASE}/Mdow-Setup.exe` }],
  },
  {
    id: 'linux',
    platform: 'Linux',
    icon: '\u{1F427}',
    formats: [{ label: 'Download .AppImage', url: `${DOWNLOAD_BASE}/Mdow.AppImage` }],
  },
]

function DownloadPage() {
  const detectedOS = Route.useLoaderData()

  const sorted = [...platforms].sort((a, b) => {
    if (a.id === detectedOS) return -1
    if (b.id === detectedOS) return 1
    return 0
  })

  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      <div className="mb-12 text-center">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Download Mdow</h1>
        <p className="mt-3 text-muted-foreground">Available for Mac, Windows, and Linux.</p>
      </div>
      <div className="grid gap-6 sm:grid-cols-3">
        {sorted.map((p) => (
          <DownloadCard
            key={p.id}
            platform={p.platform}
            icon={p.icon}
            formats={p.formats}
            recommended={p.id === detectedOS}
          />
        ))}
      </div>
    </div>
  )
}
