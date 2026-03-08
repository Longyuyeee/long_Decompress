import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  // 防止vite警告hmr端口不可用
  server: {
    hmr: {
      port: 443,
    },
  },
})