import { Link } from '@tanstack/react-router'
import { GradientSection } from '~/components/gradient-section'

const formats = ['.md', '.markdown', '.mdx']

function AppleIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z" />
    </svg>
  )
}

function WindowsIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M3 12V6.75l8-1.25V12H3zm0 .5h8v6.5l-8-1.25V12.5zM11.5 12V5.35l9.5-1.6V12h-9.5zm0 .5h9.5v8.25l-9.5-1.6V12.5z" />
    </svg>
  )
}

function LinuxIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.832-.41 1.684-.287 2.489a.424.424 0 00-.11.135c-.26.268-.45.6-.663.839-.199.199-.485.267-.797.4-.313.136-.658.269-.864.68-.09.189-.136.394-.132.602 0 .199.027.4.055.536.058.399.116.728.04.97-.249.68-.28 1.145-.106 1.484.174.334.535.47.94.601.81.2 1.91.135 2.774.6.926.466 1.866.67 2.616.47.526-.116.97-.464 1.208-.946.587-.003 1.23-.269 2.26-.334.699-.058 1.574.267 2.577.2.025.04.048.08.076.12-.184.08-.36.16-.533.24-.67.34-1.14.72-1.26 1.22-.12.5.08.95.39 1.27.31.32.72.49 1.12.57.4.08.8.08 1.09.04.58-.08 1.04-.32 1.46-.58.42-.26.8-.54 1.07-.72.27-.18.44-.24.5-.15.06.09.03.27-.12.46-.15.19-.39.39-.67.56-.28.17-.6.3-.87.37-.27.07-.49.08-.59.08-.2 0-.32-.02-.33-.02a.125.125 0 00-.07.02c-.15.09-.2.27-.15.42.05.15.17.27.32.31.15.04.32.04.44-.01.12-.05.2-.12.2-.12a.14.14 0 01.14-.01c.15.06.32.09.51.09.38 0 .79-.13 1.07-.38.28-.25.41-.6.34-.93-.07-.33-.32-.59-.6-.75-.28-.16-.6-.23-.84-.24-.24-.01-.4.03-.4.03a.126.126 0 01-.1-.03.12.12 0 01-.04-.1c0-.04.02-.09.06-.13.04-.04.1-.07.17-.07.07 0 .15.02.23.06.08.04.16.09.23.14.07.05.13.1.18.14.05.04.09.06.11.06.02 0 .03-.01.04-.03.01-.02.01-.05 0-.09a.7.7 0 00-.08-.15c-.06-.08-.14-.16-.23-.23-.09-.07-.19-.14-.29-.19-.1-.05-.2-.08-.3-.08-.1 0-.19.03-.26.08-.07.05-.12.12-.14.2-.02.08-.01.17.03.25.04.08.1.15.18.2.08.05.17.08.26.08.09 0 .17-.03.24-.08.07-.05.12-.12.14-.2.02-.08.01-.17-.03-.25z" />
    </svg>
  )
}

function GitHubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
    </svg>
  )
}

const platforms = [
  { name: 'macOS', icon: AppleIcon },
  { name: 'Windows', icon: WindowsIcon },
  { name: 'Linux', icon: LinuxIcon },
]

const GITHUB_URL = 'https://github.com/ZainW/mdow'

export function LandingTrust() {
  return (
    <GradientSection innerClassName="text-center py-16 md:py-20">
      <p className="text-sm font-medium uppercase tracking-wider text-muted-foreground">
        Free and open source
      </p>
      <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
        {formats.map((f) => (
          <span
            key={f}
            className="rounded-full border border-border-subtle bg-card px-4 py-1.5 font-mono text-xs text-muted-foreground shadow-soft"
          >
            {f}
          </span>
        ))}
      </div>
      <div className="mt-8 flex flex-wrap items-center justify-center gap-8 text-sm text-muted-foreground">
        {platforms.map((p) => (
          <div key={p.name} className="flex items-center gap-2.5">
            <p.icon className="h-4 w-4" />
            <span>{p.name}</span>
          </div>
        ))}
      </div>
      <div className="mt-10 flex flex-wrap items-center justify-center gap-4 text-sm">
        <Link
          to="/download"
          className="font-medium text-primary underline underline-offset-4 transition-colors hover:text-primary/80"
        >
          Get the latest release
        </Link>
        <span className="text-border" aria-hidden>
          ·
        </span>
        <a
          href={GITHUB_URL}
          className="inline-flex items-center gap-1.5 text-muted-foreground underline underline-offset-4 transition-colors hover:text-foreground"
          target="_blank"
          rel="noopener noreferrer"
        >
          <GitHubIcon className="h-4 w-4" />
          Star on GitHub
        </a>
      </div>
    </GradientSection>
  )
}
