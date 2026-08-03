export interface ReleaseAsset {
  arch?: 'arm64' | 'x64'
  url: string
}

export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    mac: { dmg: ReleaseAsset[]; zip: ReleaseAsset[]; gpuiBeta: ReleaseAsset | null }
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
const GPUI_ALIAS = 'mdownative-mac-beta.zip'
const GPUI_VERSIONED_SUFFIX = '-arm64-mac-beta.zip'

function detectArch(name: string): 'arm64' | 'x64' | undefined {
  if (name.includes('arm64')) return 'arm64'
  if (name.includes('x64')) return 'x64'
  return undefined
}

function releaseAsset(asset: GhAsset): ReleaseAsset {
  const arch = detectArch(asset.name)
  return arch ? { arch, url: asset.browser_download_url } : { url: asset.browser_download_url }
}

function gpuiBetaAssetType(name: string): 'alias' | 'versioned' | null {
  const normalized = name.toLowerCase()
  if (normalized === GPUI_ALIAS) return 'alias'
  if (normalized.startsWith('mdownative-') && normalized.endsWith(GPUI_VERSIONED_SUFFIX)) {
    const version = normalized.slice('mdownative-'.length, -GPUI_VERSIONED_SUFFIX.length)
    if (version.length > 0) return 'versioned'
  }
  return null
}

export function parseRelease(release: GhRelease): ReleaseInfo | null {
  if (!release?.assets?.length) return null

  const dmg: ReleaseAsset[] = []
  const zip: ReleaseAsset[] = []
  let gpuiBeta: ReleaseAsset | null = null
  let exe: string | null = null
  let appImage: string | null = null

  for (const asset of release.assets) {
    const normalized = asset.name.toLowerCase()

    if (normalized.endsWith('.dmg')) {
      dmg.push(releaseAsset(asset))
    } else if (normalized.endsWith('.zip') && normalized.includes('mac')) {
      const gpuiBetaType = gpuiBetaAssetType(asset.name)
      if (gpuiBetaType === 'alias') {
        gpuiBeta = releaseAsset(asset)
      } else if (gpuiBetaType === 'versioned') {
        gpuiBeta ??= releaseAsset(asset)
      } else {
        zip.push(releaseAsset(asset))
      }
    } else if (normalized.endsWith('.exe')) {
      exe ??= asset.browser_download_url
    } else if (normalized.endsWith('.appimage')) {
      appImage ??= asset.browser_download_url
    }
  }

  return {
    version: release.tag_name.replace(/^v/, ''),
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    assets: { mac: { dmg, zip, gpuiBeta }, windows: { exe }, linux: { appImage } },
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
