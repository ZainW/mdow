export interface ReleaseAsset {
  arch?: 'arm64' | 'x64'
  url: string
}

export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    mac: { dmg: ReleaseAsset[]; zip: ReleaseAsset[]; nativeBeta: ReleaseAsset | null }
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

function releaseAsset(asset: GhAsset): ReleaseAsset {
  const arch = detectArch(asset.name)
  return arch ? { arch, url: asset.browser_download_url } : { url: asset.browser_download_url }
}

function isNativeMacBetaAsset(name: string): boolean {
  const normalized = name.toLowerCase()
  return (
    normalized === 'mdownative-mac-beta.zip' ||
    (normalized.endsWith('.zip') &&
      normalized.includes('native') &&
      normalized.includes('mac') &&
      normalized.includes('beta'))
  )
}

export function parseRelease(release: GhRelease): ReleaseInfo | null {
  if (!release?.assets?.length) return null

  const dmg: ReleaseAsset[] = []
  const zip: ReleaseAsset[] = []
  let nativeBeta: ReleaseAsset | null = null
  let exe: string | null = null
  let appImage: string | null = null

  for (const asset of release.assets) {
    if (asset.name.endsWith('.dmg')) {
      dmg.push(releaseAsset(asset))
    } else if (asset.name.endsWith('.zip') && asset.name.includes('mac')) {
      if (isNativeMacBetaAsset(asset.name)) {
        nativeBeta ??= releaseAsset(asset)
      } else {
        zip.push(releaseAsset(asset))
      }
    } else if (asset.name.endsWith('.exe')) {
      exe ??= asset.browser_download_url
    } else if (asset.name.endsWith('.AppImage')) {
      appImage ??= asset.browser_download_url
    }
  }

  return {
    version: release.tag_name.replace(/^v/, ''),
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    assets: { mac: { dmg, zip, nativeBeta }, windows: { exe }, linux: { appImage } },
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
