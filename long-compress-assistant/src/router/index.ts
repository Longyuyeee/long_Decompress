import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    redirect: '/decompress' // 默认进入解压工作区
  },
  {
    path: '/decompress',
    name: 'Decompress',
    component: () => import('@/views/DecompressView.vue'),
    meta: {
      title: '解压工作区'
    }
  },
  {
    path: '/compress',
    name: 'Compress',
    component: () => import('@/views/CompressionView.vue'),
    meta: {
      title: '压缩工作区'
    }
  },
  {
    path: '/vault',
    name: 'Vault',
    component: () => import('@/views/PasswordVaultView.vue'),
    meta: {
      title: '密码保险箱'
    }
  },
  {
    path: '/settings',
    name: 'Settings',
    component: () => import('@/views/SettingsView.vue'),
    meta: {
      title: '全局设置'
    }
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

router.beforeEach((to, from, next) => {
  const title = to.meta?.title as string || '胧压缩'
  document.title = `${title} - 胧压缩`
  next()
})

export default router
