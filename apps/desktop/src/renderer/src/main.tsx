import React from 'react'
import ReactDOM from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import App from './App'
import { initMarkdown } from './lib/markdown'
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

function scheduleMarkdownWarmup(): void {
  const run = () => {
    void initMarkdown()
  }
  if ('requestIdleCallback' in globalThis) {
    requestIdleCallback(run)
  } else {
    setTimeout(run, 1)
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
