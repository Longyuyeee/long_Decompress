/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}

// Vue i18n types - custom translation function
declare module '@vue/runtime-core' {
  interface ComponentCustomProperties {
    $t: (key: string, fallback?: string) => string
  }
}

// Tauri API types
declare module '@tauri-apps/api/tauri' {
  export function invoke<T = any>(cmd: string, args?: Record<string, any>): Promise<T>
}

declare global {
  interface Window {
    __TAURI__?: {
      tauri: {
        invoke<T = any>(cmd: string, args?: Record<string, any>): Promise<T>
      }
    }
  }
}