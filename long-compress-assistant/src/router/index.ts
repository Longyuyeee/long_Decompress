import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: Array<RouteRecordRaw> = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/HomeView.vue'),
    meta: {
      title: '胧压缩·方便助手'
    }
  },
  {
    path: '/decompress',
    name: 'Decompress',
    component: () => import('@/views/DecompressView.vue'),
    meta: {
      title: '文件解压'
    }
  },
  {
    path: '/compress',
    name: 'Compress',
    component: () => import('@/views/CompressView.vue'),
    meta: {
      title: '文件压缩'
    }
  },
  {
    path: '/settings',
    name: 'Settings',
    component: () => import('@/views/SettingsView.vue'),
    meta: {
      title: '设置'
    }
  },
  {
    path: '/about',
    name: 'About',
    component: () => import('@/views/AboutView.vue'),
    meta: {
      title: '关于'
    }
  },
  {
    path: '/design-system',
    name: 'DesignSystem',
    component: () => import('@/components/ui/DesignSystemShowcase.vue'),
    meta: {
      title: '设计系统'
    }
  },
  {
    path: '/tasks',
    name: 'Tasks',
    component: () => import('@/views/TasksView.vue'),
    meta: {
      title: '任务管理'
    }
  },
  {
    path: '/passwords',
    name: 'Passwords',
    component: () => import('@/views/PasswordsView.vue'),
    meta: {
      title: '密码本管理'
    }
  },
  {
    path: '/responsive-test',
    name: 'ResponsiveTest',
    component: () => import('@/views/ResponsiveTestView.vue'),
    meta: {
      title: '响应式设计测试'
    }
  },
  {
    path: '/animation-test',
    name: 'AnimationTest',
    component: () => import('@/views/AnimationTestView.vue'),
    meta: {
      title: '动画效果测试'
    }
  }
  ]

  const router = createRouter({
  history: createWebHistory(),
  routes
  })

  // 路由守卫：更新页面标题
  router.beforeEach((to, from, next) => {
  const title = to.meta?.title as string || '胧压缩·方便助手'
  document.title = `${title} - 胧压缩·方便助手`
  next()
  })

export default router
