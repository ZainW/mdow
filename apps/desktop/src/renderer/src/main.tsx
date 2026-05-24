import React from 'react'
import ReactDOM from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import App from './App'
import { installNativeApi } from '@renderer/lib/native-api'
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

function renderFailure(error: unknown): void {
  console.error('Failed to install native API:', error)
  const root = document.getElementById('root')
  if (!root) return

  root.textContent = 'Failed to initialize Mdow. Check the console for details.'
}

function renderApp(): void {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </React.StrictMode>,
  )
}

void installNativeApi().then(renderApp).catch(renderFailure)
