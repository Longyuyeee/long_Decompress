<template>
  <div class="min-h-screen flex flex-col bg-gradient-to-br from-gray-50 to-blue-50 dark:from-gray-900 dark:to-blue-900">
    <!-- 跳过导航链接（可访问性） -->
    <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2 focus:bg-primary focus:text-white focus:rounded-lg focus:shadow-lg">
      跳过导航
    </a>

    <!-- 顶部导航栏 -->
    <header class="sticky top-0 z-50 backdrop-blur-xl bg-white/80 dark:bg-gray-900/80 border-b border-gray-200 dark:border-gray-800 supports-[backdrop-filter]:bg-white/60 supports-[backdrop-filter]:dark:bg-gray-900/60">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex items-center justify-between h-16">
          <!-- 左侧：Logo和导航 -->
          <div class="flex items-center">
            <router-link to="/" class="flex items-center space-x-2 sm:space-x-3">
              <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center flex-shrink-0">
                <i class="pi pi-box text-white text-sm"></i>
              </div>
              <div class="hidden sm:block">
                <h1 class="text-lg font-bold text-gray-900 dark:text-white truncate">胧压缩·方便助手</h1>
                <p class="text-xs text-gray-500 dark:text-gray-400 hidden md:block">智能文件解压工具</p>
              </div>
              <div class="sm:hidden">
                <h1 class="text-base font-bold text-gray-900 dark:text-white">胧压缩</h1>
              </div>
            </router-link>

            <!-- 桌面端导航 -->
            <nav class="hidden md:flex ml-6 lg:ml-10 space-x-1" aria-label="主导航">
              <router-link
                v-for="item in navItems"
                :key="item.to"
                :to="item.to"
                class="px-3 py-2 rounded-md text-sm font-medium transition-colors hover:scale-105 active:scale-95"
                :class="[
                  $route.path === item.to
                    ? 'bg-primary/10 text-primary shadow-sm'
                    : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
                ]"
              >
                <i :class="['pi', item.icon, 'mr-2']"></i>
                {{ item.label }}
              </router-link>
            </nav>
          </div>

          <!-- 右侧：工具和用户 -->
          <div class="flex items-center space-x-2 sm:space-x-4">
            <!-- 搜索框 -->
            <div class="hidden md:block relative">
              <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <i class="pi pi-search text-gray-400"></i>
              </div>
              <input
                type="text"
                placeholder="搜索文件或设置..."
                class="pl-10 pr-4 py-2 w-48 lg:w-64 rounded-lg border border-gray-300 dark:border-gray-600 bg-white/50 dark:bg-gray-800/50 focus:outline-none focus:ring-2 focus:ring-primary/50 focus:border-primary/50 transition-all"
              />
            </div>

            <!-- 移动端搜索按钮 -->
            <button class="md:hidden p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors" title="搜索">
              <i class="pi pi-search text-gray-700 dark:text-gray-300"></i>
            </button>

            <!-- 主题切换 -->
            <button
              @click="toggleTheme"
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors hover:scale-105 active:scale-95"
              :title="currentTheme === 'dark' ? '切换到亮色主题' : '切换到暗色主题'"
            >
              <i
                :class="[
                  'pi text-lg',
                  currentTheme === 'dark' ? 'pi-sun text-yellow-500' : 'pi-moon text-gray-700 dark:text-gray-300'
                ]"
              ></i>
            </button>

            <!-- 通知 -->
            <button
              class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors hover:scale-105 active:scale-95 relative"
              @click="toggleNotifications"
              :title="`通知 (${unreadNotifications})`"
            >
              <i class="pi pi-bell text-gray-700 dark:text-gray-300 text-lg"></i>
              <span
                v-if="unreadNotifications > 0"
                class="absolute -top-1 -right-1 w-5 h-5 bg-red-500 text-white text-xs rounded-full flex items-center justify-center animate-pulse"
              >
                {{ unreadNotifications > 9 ? '9+' : unreadNotifications }}
              </span>
            </button>

            <!-- 用户菜单 -->
            <div class="relative">
              <button
                @click="toggleUserMenu"
                class="flex items-center space-x-2 p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors hover:scale-105 active:scale-95"
                :title="userName"
              >
                <div class="w-8 h-8 rounded-full bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center flex-shrink-0">
                  <i class="pi pi-user text-white text-sm"></i>
                </div>
                <span class="hidden md:block text-sm font-medium text-gray-700 dark:text-gray-300 truncate max-w-24">
                  {{ userName }}
                </span>
                <i class="pi pi-chevron-down text-gray-500 hidden sm:block"></i>
              </button>

              <!-- 用户菜单下拉 -->
              <div
                v-if="showUserMenu"
                class="absolute right-0 mt-2 w-48 rounded-lg shadow-lg glass-effect border border-gray-200 dark:border-gray-700 py-1 z-50"
              >
                <div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
                  <p class="text-sm font-medium text-gray-900 dark:text-white">{{ userName }}</p>
                  <p class="text-xs text-gray-500 dark:text-gray-400">管理员</p>
                </div>
                <div class="py-1">
                  <router-link
                    to="/settings"
                    class="flex items-center px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                    @click="showUserMenu = false"
                  >
                    <i class="pi pi-cog mr-3"></i>
                    设置
                  </router-link>
                  <router-link
                    to="/about"
                    class="flex items-center px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                    @click="showUserMenu = false"
                  >
                    <i class="pi pi-info-circle mr-3"></i>
                    关于
                  </router-link>
                </div>
                <div class="py-1 border-t border-gray-200 dark:border-gray-700">
                  <button
                    @click="logout"
                    class="flex items-center w-full px-4 py-2 text-sm text-red-600 dark:text-red-400 hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    <i class="pi pi-sign-out mr-3"></i>
                    退出登录
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 移动端导航 -->
      <div class="md:hidden border-t border-gray-200 dark:border-gray-800">
        <div class="px-2 pt-2 pb-3 space-y-1">
          <!-- 移动端侧边栏菜单按钮 -->
          <button
            @click="openMobileSidebar"
            class="w-full flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            <i class="pi pi-bars mr-3"></i>
            菜单
          </button>

          <router-link
            v-for="item in navItems"
            :key="item.to"
            :to="item.to"
            class="flex items-center px-3 py-2 rounded-md text-base font-medium"
            :class="[
              $route.path === item.to
                ? 'bg-primary/10 text-primary'
                : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
            ]"
          >
            <i :class="['pi', item.icon, 'mr-3']"></i>
            {{ item.label }}
          </router-link>
        </div>
      </div>
    </header>

    <!-- 主要内容区域 -->
    <div class="flex-1 flex">
      <!-- 侧边栏 -->
      <Sidebar ref="sidebarRef" />

      <!-- 主内容区 -->
      <main id="main-content" class="flex-1 overflow-auto animate-fade-in" tabindex="-1">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4 sm:py-6">
          <!-- 面包屑导航和移动端菜单按钮 -->
          <div class="flex items-center justify-between mb-4 sm:mb-6">
            <div class="flex items-center space-x-1 sm:space-x-2">
              <!-- 移动端菜单按钮 -->
              <button
                @click="openMobileSidebar"
                class="lg:hidden p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 mr-1 sm:mr-2 hover:scale-105 active:scale-95 transition-transform"
                aria-label="打开菜单"
              >
                <i class="pi pi-bars text-gray-700 dark:text-gray-300"></i>
              </button>

              <!-- 面包屑导航 -->
              <nav class="flex items-center space-x-1 sm:space-x-2 text-xs sm:text-sm text-gray-600 dark:text-gray-400" aria-label="面包屑导航">
                <router-link to="/" class="hover:text-primary transition-colors p-1 rounded" aria-label="首页">
                  <i class="pi pi-home"></i>
                </router-link>
                <i class="pi pi-chevron-right text-xs opacity-50"></i>

                <!-- 动态面包屑 -->
                <template v-for="(crumb, index) in breadcrumbs" :key="crumb.path">
                  <router-link
                    v-if="index < breadcrumbs.length - 1"
                    :to="crumb.path"
                    class="hover:text-primary transition-colors px-1 py-0.5 rounded truncate max-w-20 sm:max-w-none"
                    :aria-label="`前往${crumb.title}`"
                  >
                    {{ crumb.title }}
                  </router-link>
                  <span v-else class="text-gray-900 dark:text-white font-medium px-1 py-0.5 truncate max-w-32 sm:max-w-none" aria-current="page">
                    {{ crumb.title }}
                  </span>

                  <i v-if="index < breadcrumbs.length - 1" class="pi pi-chevron-right text-xs opacity-50"></i>
                </template>
              </nav>
            </div>

            <!-- 快捷操作 -->
            <div class="flex items-center space-x-1 sm:space-x-2">
              <button class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors hover:scale-105 active:scale-95" title="刷新" aria-label="刷新页面">
                <i class="pi pi-refresh text-gray-700 dark:text-gray-300"></i>
              </button>
              <button class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors hover:scale-105 active:scale-95" title="帮助" aria-label="打开帮助">
                <i class="pi pi-question-circle text-gray-700 dark:text-gray-300"></i>
              </button>
            </div>
          </div>

          <!-- 页面内容 -->
          <div class="animate-slide-up">
            <slot />
          </div>
        </div>
      </main>
    </div>

    <!-- 底部信息 -->
    <footer class="border-t border-gray-200 dark:border-gray-800 bg-white/50 dark:bg-gray-900/50 supports-[backdrop-filter]:bg-white/30 supports-[backdrop-filter]:dark:bg-gray-900/30">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-3 sm:py-4">
        <div class="flex flex-col md:flex-row items-center justify-between space-y-3 md:space-y-0">
          <div class="text-xs sm:text-sm text-gray-600 dark:text-gray-400 text-center md:text-left">
            © 2024 胧压缩·方便助手. 版本 {{ appVersion }}
          </div>
          <div class="flex flex-wrap items-center justify-center gap-3 sm:gap-6">
            <router-link to="/about" class="text-xs sm:text-sm text-gray-600 dark:text-gray-400 hover:text-primary transition-colors px-1 py-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800">
              关于我们
            </router-link>
            <a href="#" class="text-xs sm:text-sm text-gray-600 dark:text-gray-400 hover:text-primary transition-colors px-1 py-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800">
              帮助文档
            </a>
            <a href="#" class="text-xs sm:text-sm text-gray-600 dark:text-gray-400 hover:text-primary transition-colors px-1 py-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800">
              隐私政策
            </a>
            <a href="#" class="text-xs sm:text-sm text-gray-600 dark:text-gray-400 hover:text-primary transition-colors px-1 py-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800">
              服务条款
            </a>
          </div>
        </div>
      </div>
    </footer>

    <!-- 通知面板 -->
    <div
      v-if="showNotifications"
      class="fixed inset-0 z-50 overflow-hidden"
      @click="showNotifications = false"
    >
      <div class="absolute inset-0 bg-black/50 animate-fade-in"></div>
      <div
        class="absolute right-0 top-16 bottom-0 w-full sm:w-96 bg-white dark:bg-gray-900 shadow-xl animate-slide-left"
        @click.stop
      >
        <div class="h-full flex flex-col">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-800">
            <div class="flex items-center justify-between">
              <h3 class="text-lg font-semibold text-gray-900 dark:text-white">通知</h3>
              <button
                @click="showNotifications = false"
                class="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800"
              >
                <i class="pi pi-times text-gray-500"></i>
              </button>
            </div>
          </div>
          <div class="flex-1 overflow-y-auto p-4">
            <div v-if="notifications.length === 0" class="text-center py-8">
              <i class="pi pi-bell text-gray-400 text-4xl mb-4"></i>
              <p class="text-gray-500 dark:text-gray-400">暂无通知</p>
            </div>
            <div v-else class="space-y-3">
              <div
                v-for="notification in notifications"
                :key="notification.id"
                class="p-3 rounded-lg border border-gray-200 dark:border-gray-700"
                :class="{
                  'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800': notification.type === 'info',
                  'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800': notification.type === 'success',
                  'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800': notification.type === 'warning',
                  'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800': notification.type === 'error'
                }"
              >
                <div class="flex items-start">
                  <i
                    :class="[
                      'pi mt-0.5 mr-3',
                      notification.type === 'info' ? 'pi-info-circle text-blue-500' : '',
                      notification.type === 'success' ? 'pi-check-circle text-green-500' : '',
                      notification.type === 'warning' ? 'pi-exclamation-triangle text-yellow-500' : '',
                      notification.type === 'error' ? 'pi-times-circle text-red-500' : ''
                    ]"
                  ></i>
                  <div class="flex-1">
                    <p class="font-medium text-gray-900 dark:text-white">{{ notification.title }}</p>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">{{ notification.message }}</p>
                    <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">
                      {{ formatRelativeTime(notification.createdAt) }}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800">
            <button
              @click="clearNotifications"
              class="w-full py-2 px-4 rounded-lg border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
            >
              清除所有通知
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { formatRelativeTime } from '@/utils'
import Sidebar from './Sidebar.vue'

interface NavItem {
  to: string
  label: string
  icon: string
}

const route = useRoute()
const appStore = useAppStore()
const sidebarRef = ref<InstanceType<typeof Sidebar> | null>(null)

// 状态
const showUserMenu = ref(false)
const showNotifications = ref(false)
const notifications = ref<Array<{
  id: string
  type: 'info' | 'success' | 'warning' | 'error'
  title: string
  message: string
  createdAt: Date
}>>([
  {
    id: '1',
    type: 'info',
    title: '欢迎使用',
    message: '欢迎使用胧压缩·方便助手！',
    createdAt: new Date(Date.now() - 3600000) // 1小时前
  },
  {
    id: '2',
    type: 'success',
    title: '解压完成',
    message: '文件 archive.zip 解压成功',
    createdAt: new Date(Date.now() - 1800000) // 30分钟前
  }
])

// 计算属性
const currentTheme = computed(() => appStore.currentTheme)

// 面包屑导航
const breadcrumbs = computed(() => {
  const crumbs: Array<{ path: string; title: string }> = []

  // 首页总是第一个
  crumbs.push({ path: '/', title: '首页' })

  // 根据当前路由添加面包屑
  const routeName = route.name?.toString() || ''
  switch (routeName) {
    case 'Home':
      // 首页已经在上面添加了
      break
    case 'Decompress':
      crumbs.push({ path: '/decompress', title: '文件解压' })
      break
    case 'Settings':
      crumbs.push({ path: '/settings', title: '设置' })
      break
    case 'About':
      crumbs.push({ path: '/about', title: '关于' })
      break
    case 'Tasks':
      crumbs.push({ path: '/tasks', title: '任务管理' })
      break
    default:
      if (route.path !== '/') {
        crumbs.push({ path: route.path, title: currentPageTitle.value })
      }
  }

  return crumbs
})

const currentPageTitle = computed(() => {
  const routeName = route.name?.toString() || ''
  switch (routeName) {
    case 'Home':
      return '首页'
    case 'Decompress':
      return '文件解压'
    case 'Tasks':
      return '任务管理'
    case 'Settings':
      return '设置'
    case 'About':
      return '关于'
    default:
      return '页面'
  }
})

const unreadNotifications = computed(() => {
  return notifications.value.length
})

const userName = computed(() => '用户')
const appVersion = computed(() => '1.0.0')

const navItems = computed<NavItem[]>(() => [
  { to: '/', label: '首页', icon: 'pi-home' },
  { to: '/decompress', label: '解压', icon: 'pi-file-import' },
  { to: '/tasks', label: '任务', icon: 'pi-list' },
  { to: '/design-system', label: '设计系统', icon: 'pi-palette' },
  { to: '/settings', label: '设置', icon: 'pi-cog' }
])

// 方法
const toggleTheme = () => {
  const newTheme = currentTheme.value === 'dark' ? 'light' : 'dark'
  appStore.updateSettings({ theme: newTheme })
}

const toggleUserMenu = () => {
  showUserMenu.value = !showUserMenu.value
  showNotifications.value = false
}

const toggleNotifications = () => {
  showNotifications.value = !showNotifications.value
  showUserMenu.value = false
}

const logout = () => {
  console.log('退出登录')
  showUserMenu.value = false
  // 这里应该调用退出登录的API
}

const clearNotifications = () => {
  notifications.value = []
  showNotifications.value = false
}

const openMobileSidebar = () => {
  if (sidebarRef.value) {
    sidebarRef.value.openMobileSidebar()
  }
}

const closeMobileSidebar = () => {
  if (sidebarRef.value) {
    sidebarRef.value.closeMobileSidebar()
  }
}

const closeMenus = (event: MouseEvent) => {
  const target = event.target as HTMLElement
  if (!target.closest('.user-menu') && !target.closest('.notifications-menu')) {
    showUserMenu.value = false
    showNotifications.value = false
  }
}

// 生命周期
onMounted(() => {
  document.addEventListener('click', closeMenus)
})

onUnmounted(() => {
  document.removeEventListener('click', closeMenus)
})
</script>

<style scoped>
.router-link-active {
  @apply bg-primary/10 text-primary;
}

.user-menu {
  position: relative;
}

.notifications-menu {
  position: relative;
}

/* 滚动条样式 */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  @apply bg-transparent;
}

::-webkit-scrollbar-thumb {
  @apply bg-gray-300 dark:bg-gray-700 rounded-full;
}

::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-400 dark:bg-gray-600;
}
</style>