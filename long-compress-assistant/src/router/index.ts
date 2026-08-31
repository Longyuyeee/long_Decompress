import { createRouter, createWebHashHistory } from 'vue-router'
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
    path: '/special-compression',
    name: 'SpecialCompression',
    component: () => import('@/views/SpecialCompressionView.vue'),
    meta: {
      title: '特殊压缩'
    }
  },
  {
    path: '/browser',
    name: 'ArchiveBrowser',
    component: () => import('@/views/ArchiveBrowserView.vue'),
    meta: {
      title: '压缩包浏览中心'
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
    path: '/history',
    name: 'History',
    component: () => import('@/views/HistoryView.vue'),
    meta: {
      title: '历史任务'
    }
  },
  {
    path: '/settings',
    name: 'Settings',
    component: () => import('@/views/SettingsView.vue'),
    meta: {
      title: '全局设置'
    }
  },
  {
    path: '/integrity',
    name: 'FileIntegrity',
    component: () => import('@/views/FileIntegrityView.vue'),
    meta: {
      title: '文件完整性校验'
    }
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'NotFound',
    redirect: '/decompress'
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

router.beforeEach((to, from, next) => {
  const title = to.meta?.title as string || 'Long解压'
  document.title = `${title} - Long解压`
  next()
})

export default router
