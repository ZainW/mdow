import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function isMacPlatform(): boolean {
  return window.api?.platform === 'darwin'
}

export const isMac = typeof window !== 'undefined' && window.api?.platform === 'darwin'
