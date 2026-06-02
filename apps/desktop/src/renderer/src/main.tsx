import React from 'react'
import ReactDOM from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import App from './App'
import './assets/styles/index.css'
import './assets/styles/markdown.css'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
  },
})

function runMarkdownWarmup(): void {
  void import('./lib/markdown').then(({ initMarkdown }) => initMarkdown())
}

function scheduleMarkdownWarmup(): void {
  if ('requestIdleCallback' in globalThis) {
    requestIdleCallback(runMarkdownWarmup)
  } else {
    setTimeout(runMarkdownWarmup, 1)
  }
}

scheduleMarkdownWarmup()

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
)
