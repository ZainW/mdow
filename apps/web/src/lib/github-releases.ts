export interface ReleaseAsset {
  arch?: 'arm64' | 'x64'
  url: string
}

export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    mac: { dmg: ReleaseAsset[]; zip: ReleaseAsset[] }
    windows: { exe: string | null }
    linux: { appImage: string | null }
  }
}

interface GhAsset {
  name: string
  browser_download_url: string
}

interface GhRelease {
  tag_name: string
  name?: string
  published_at: string
  html_url: string
  assets: GhAsset[]
}

const REPO = 'ZainW/mdow'

function detectArch(name: string): 'arm64' | 'x64' | undefined {
  if (name.includes('arm64')) return 'arm64'
  if (name.includes('x64')) return 'x64'
  return undefined
}

export function parseRelease(release: GhRelease): ReleaseInfo | null {
  if (!release?.assets?.length) return null

  const dmg = release.assets
    .filter((a) => a.name.endsWith('.dmg'))
    .map((a) => ({ arch: detectArch(a.name), url: a.browser_download_url }))

  const zip = release.assets
    .filter((a) => a.name.endsWith('.zip') && a.name.includes('mac'))
    .map((a) => ({ arch: detectArch(a.name), url: a.browser_download_url }))

  const exe = release.assets.find((a) => a.name.endsWith('.exe'))?.browser_download_url ?? null

  const appImage =
    release.assets.find((a) => a.name.endsWith('.AppImage'))?.browser_download_url ?? null

  return {
    version: release.tag_name.replace(/^v/, ''),
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    assets: { mac: { dmg, zip }, windows: { exe }, linux: { appImage } },
  }
}

export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: {
        Accept: 'application/vnd.github+json',
        'User-Agent': 'mdow-website',
      },
    })
    if (!res.ok) return null
    const json = (await res.json()) as GhRelease
    return parseRelease(json)
  } catch {
    return null
  }
}
