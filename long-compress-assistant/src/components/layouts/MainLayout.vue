<template>
  <div class="min-h-screen flex flex-col bg-gradient-to-br from-gray-50 to-blue-50 dark:from-gray-900 dark:to-blue-900">
    <!-- 跳过导航链接（可访问性） -->
    <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2 focus:bg-primary focus:text-white focus:rounded-lg focus:shadow-lg">
      跳过导航
    </a>

    <!-- 顶部导航栏 -->
    <header class="sticky top-0 z-50 backdrop-blur-xl bg-white/80 dark:bg-gray-900/80 border-b border-gray-200 dark:border-gray-800">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex items-center justify-between h-16">
          <div class="flex items-center">
            <router-link to="/" class="flex items-center space-x-2 sm:space-x-3">
              <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center flex-shrink-0">
                <i class="pi pi-box text-white text-sm"></i>
              </div>
              <div class="hidden sm:block">
                <h1 class="text-lg font-bold text-gray-900 dark:text-white truncate">胧压缩·方便助手</h1>
                <p class="text-xs text-gray-500 dark:text-gray-400 hidden md:block">智能文件解压工具</p>
              </div>
            </router-link>

            <nav class="hidden md:flex ml-6 lg:ml-10 space-x-1">
              <router-link
                v-for="item in navItems"
                :key="item.to"
                :to="item.to"
                class="px-3 py-2 rounded-md text-sm font-medium transition-all"
                :class="[
                  $route.path === item.to
                    ? 'bg-primary/10 text-primary'
                    : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
                ]"
              >
                <i :class="['pi', item.icon, 'mr-2']"></i>
                {{ item.label }}
              </router-link>
            </nav>
          </div>

          <div class="flex items-center space-x-2 sm:space-x-4">
            <ThemeToggle variant="icon" />
            
            <!-- 通知按钮 -->
            <button class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 relative" @click="toggleNotifications">
              <i class="pi pi-bell text-gray-700 dark:text-gray-300 text-lg"></i>
              <span v-if="unreadNotifications > 0" class="absolute -top-1 -right-1 w-5 h-5 bg-red-500 text-white text-xs rounded-full flex items-center justify-center">
                {{ unreadNotifications }}
              </span>
            </button>

            <!-- 用户菜单 -->
            <div class="relative">
              <button @click="toggleUserMenu" class="flex items-center space-x-2 p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800">
                <div class="w-8 h-8 rounded-full bg-primary flex items-center justify-center text-white">
                  <i class="pi pi-user text-sm"></i>
                </div>
                <span class="hidden md:block text-sm font-medium">{{ userName }}</span>
              </button>

              <div v-if="showUserMenu" class="absolute right-0 mt-2 w-48 rounded-lg shadow-lg bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 py-1 z-50">
                <div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 text-sm">
                  <p class="font-medium text-gray-900 dark:text-white">{{ userName }}</p>
                  <p class="text-xs text-gray-500">管理员</p>
                </div>
                <router-link to="/settings" class="block px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700" @click="showUserMenu = false">设置</router-link>
                <button @click="logout" class="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-gray-100 dark:hover:bg-gray-700 border-t border-gray-200 dark:border-gray-700">
                  退出登录
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </header>

    <div class="flex-1 flex overflow-hidden">
      <!-- 侧边栏 -->
      <Sidebar ref="sidebarRef" />

      <!-- 主内容区 -->
      <main id="main-content" class="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
        <div class="max-w-7xl mx-auto">
          <!-- 面包屑 -->
          <nav class="flex items-center space-x-2 text-sm text-gray-500 mb-6">
            <router-link to="/" class="hover:text-primary"><i class="pi pi-home"></i></router-link>
            <template v-for="crumb in breadcrumbs" :key="crumb.path">
              <i class="pi pi-chevron-right text-xs opacity-50"></i>
              <router-link :to="crumb.path" class="hover:text-primary">{{ crumb.title }}</router-link>
            </template>
          </nav>

          <slot />
        </div>
      </main>
    </div>

    <!-- 页脚 -->
    <footer class="border-t border-gray-200 dark:border-gray-800 bg-white/50 dark:bg-gray-900/50 py-4">
      <div class="max-w-7xl mx-auto px-4 text-center text-xs text-gray-500">
        © 2024 胧压缩·方便助手 | 版本 {{ appVersion }}
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'
import Sidebar from './Sidebar.vue'
import ThemeToggle from '@/components/ui/ThemeToggle.vue'

interface NavItem {
  to: string
  label: string
  icon: string
}

const route = useRoute()
const appStore = useAppStore()
const sidebarRef = ref<any>(null)

const showUserMenu = ref(false)
const showNotifications = ref(false)
const userName = ref('用户')
const appVersion = ref('1.0.0')

const navItems = computed<NavItem[]>(() => [
  { to: '/', label: '首页', icon: 'pi-home' },
  { to: '/decompress', label: '文件解压', icon: 'pi-file-import' },
  { to: '/passwords', label: '密码管理', icon: 'pi-lock' },
  { to: '/tasks', label: '任务列表', icon: 'pi-list' },
  { to: '/settings', label: '设置', icon: 'pi-cog' }
])

const unreadNotifications = ref(2)

const breadcrumbs = computed(() => {
  const crumbs: Array<{ path: string; title: string }> = []
  const pathParts = route.path.split('/').filter(p => p)
  
  let currentPath = ''
  pathParts.forEach(part => {
    currentPath += `/${part}`
    let title = part
    if (part === 'decompress') title = '解压'
    else if (part === 'passwords') title = '密码管理'
    else if (part === 'tasks') title = '任务列表'
    else if (part === 'settings') title = '设置'
    crumbs.push({ path: currentPath, title })
  })
  
  return crumbs
})

const toggleUserMenu = () => { showUserMenu.value = !showUserMenu.value }
const toggleNotifications = () => { showNotifications.value = !showNotifications.value }
const logout = () => { console.log('退出登录'); showUserMenu.value = false }

const openMobileSidebar = () => sidebarRef.value?.openMobileSidebar()

const closeMenus = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.user-menu')) showUserMenu.value = false
}

onMounted(() => document.addEventListener('click', closeMenus))
onUnmounted(() => document.removeEventListener('click', closeMenus))
</script>

<style scoped>
.router-link-active {
  @apply bg-primary/10 text-primary;
}
</style>
