/// <reference types="vite/client" />
import type { ReactNode } from 'react'
import { Outlet, createRootRoute, HeadContent, Scripts, Link } from '@tanstack/react-router'
import appCss from '~/styles/app.css?url'
import interFont from '~/assets/fonts/InterVariable.woff2?url'
import newsreaderFont from '~/assets/fonts/Newsreader-Variable.woff2?url'
import { btnPrimary, btnSecondary } from '~/lib/button-styles'
import { seo } from '~/lib/seo'
import { SiteHeader } from '~/components/site-header'
import { SiteFooter } from '~/components/site-footer'

const THEME_SCRIPT = `(function(){try{var t=localStorage.getItem('theme');var d=t==='dark'||(!t&&matchMedia('(prefers-color-scheme:dark)').matches);if(d)document.documentElement.classList.add('dark')}catch(e){}})()`

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1, viewport-fit=cover' },
      { name: 'theme-color', content: '#faf8f5', media: '(prefers-color-scheme: light)' },
      { name: 'theme-color', content: '#1b1a19', media: '(prefers-color-scheme: dark)' },
      ...seo({
        title: 'Mdow: A Quiet Markdown Reader for Mac, Windows & Linux',
        description:
          'A fast, focused markdown reader for your notes, docs, and READMEs. Browse folders, render Mermaid and Shiki, and ask questions with OpenCode or Codex. Free and open source.',
      }),
    ],
    links: [
      { rel: 'stylesheet', href: appCss },
      { rel: 'icon', href: '/favicon.ico' },
      { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32x32.png' },
      { rel: 'icon', type: 'image/png', sizes: '16x16', href: '/favicon-16x16.png' },
      { rel: 'apple-touch-icon', sizes: '180x180', href: '/apple-touch-icon.png' },
      { rel: 'preload', as: 'font', type: 'font/woff2', href: interFont, crossOrigin: 'anonymous' },
      {
        rel: 'preload',
        as: 'font',
        type: 'font/woff2',
        href: newsreaderFont,
        crossOrigin: 'anonymous',
      },
      {
        rel: 'alternate',
        type: 'application/rss+xml',
        title: 'Mdow Changelog',
        href: '/changelog/rss.xml',
      },
    ],
    scripts: [{ children: THEME_SCRIPT }],
  }),
  component: RootComponent,
  notFoundComponent: NotFound,
})

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  )
}

function NotFound() {
  return (
    <RootDocument>
      <div className="mx-auto flex min-h-[65vh] max-w-xl flex-col items-center justify-center px-5 py-20 text-center">
        <p className="font-mono text-sm text-muted-foreground">404</p>
        <h1 className="font-display mt-4 text-5xl">This page isn&rsquo;t here.</h1>
        <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
          It may have moved, or the link might be mistyped.
        </p>
        <div className="mt-8 flex flex-col gap-3 sm:flex-row">
          <Link to="/" className={btnPrimary('md')}>
            Back to home
          </Link>
          <Link to="/docs" className={btnSecondary('md')}>
            Browse the docs
          </Link>
        </div>
      </div>
    </RootDocument>
  )
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body className="flex min-h-screen flex-col font-sans">
        <a
          href="#main"
          className="sr-only z-modal rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground focus:not-sr-only focus:fixed focus:left-4 focus:top-4"
        >
          Skip to content
        </a>
        <SiteHeader />
        <main id="main" className="flex-1">
          {children}
        </main>
        <SiteFooter />
        <Scripts />
      </body>
    </html>
  )
}
