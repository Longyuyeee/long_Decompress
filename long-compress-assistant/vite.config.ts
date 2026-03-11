import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  // Vite 选项用于 Tauri 开发
  // 1. 防止 Vite 忽略环境变量中的 TAURI_PLATFORM 等变量
  clearScreen: false,
  // 2. Tauri 预期使用指定的端口，若端口不可用则直接报错而不是跳过
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // 3. 告诉 Vite 忽略 src-tauri
      ignored: ["**/src-tauri/**"],
    },
  },
})