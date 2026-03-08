<template>
  <aside class="hidden lg:flex flex-col w-64 border-r border-gray-200 dark:border-gray-800 bg-white/50 dark:bg-gray-900/50 backdrop-blur-sm">
    <!-- 侧边栏头部 -->
    <div class="p-6 border-b border-gray-200 dark:border-gray-800">
      <div class="flex items-center space-x-3">
        <div class="w-10 h-10 rounded-lg bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center">
          <i class="pi pi-folder text-white"></i>
        </div>
        <div>
          <h3 class="font-semibold text-gray-900 dark:text-white">文件管理</h3>
          <p class="text-xs text-gray-500 dark:text-gray-400">快速访问</p>
        </div>
      </div>
    </div>

    <!-- 导航菜单 -->
    <nav class="flex-1 p-4 overflow-y-auto">
      <div class="space-y-1">
        <div class="px-3 py-2">
          <h4 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">常用功能</h4>
          <div class="space-y-1">
            <router-link
              to="/decompress"
              class="flex items-center px-3 py-2 rounded-lg text-sm transition-colors"
              :class="[
                $route.path === '/decompress'
                  ? 'bg-primary/10 text-primary'
                  : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
              ]"
            >
              <i class="pi pi-file-import mr-3"></i>
              文件解压
            </router-link>
            <router-link
              to="/compress"
              class="flex items-center px-3 py-2 rounded-lg text-sm transition-colors"
              :class="[
                $route.path === '/compress'
                  ? 'bg-primary/10 text-primary'
                  : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
              ]"
            >
              <i class="pi pi-compress mr-3"></i>
              文件压缩
            </router-link>
            <button class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-eye mr-3"></i>
              文件预览
            </button>
          </div>
        </div>

        <div class="px-3 py-2">
          <h4 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">文件管理</h4>
          <div class="space-y-1">
            <button class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-history mr-3"></i>
              最近文件
              <span class="ml-auto text-xs px-2 py-1 bg-gray-200 dark:bg-gray-700 rounded-full">12</span>
            </button>
            <button class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-star mr-3"></i>
              收藏夹
              <span class="ml-auto text-xs px-2 py-1 bg-gray-200 dark:bg-gray-700 rounded-full">8</span>
            </button>
            <button class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-trash mr-3"></i>
              回收站
            </button>
          </div>
        </div>

        <div class="px-3 py-2">
          <h4 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">工具</h4>
          <div class="space-y-1">
            <button class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-shield mr-3"></i>
              安全检查
            </button>
            <button class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-chart-bar mr-3"></i>
              统计信息
            </button>
            <button class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-wrench mr-3"></i>
              批量处理
            </button>
          </div>
        </div>
      </div>
    </nav>

    <!-- 侧边栏底部 -->
    <div class="p-4 border-t border-gray-200 dark:border-gray-800">
      <div class="space-y-3">
        <div class="px-3">
          <div class="flex items-center justify-between mb-1">
            <span class="text-xs text-gray-600 dark:text-gray-400">存储空间</span>
            <span class="text-xs font-medium text-gray-900 dark:text-white">23%</span>
          </div>
          <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
            <div class="bg-primary h-1.5 rounded-full" style="width: 23%"></div>
          </div>
        </div>
        <button class="w-full flex items-center justify-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
          <i class="pi pi-cog mr-2"></i>
          管理存储
        </button>
      </div>
    </div>
  </aside>

  <!-- 移动端侧边栏 -->
  <div
    v-if="mobileSidebarOpen"
    class="fixed inset-0 z-40 lg:hidden"
    @click="closeMobileSidebar"
  >
    <div class="absolute inset-0 bg-black/50"></div>
    <div
      class="absolute left-0 top-0 bottom-0 w-64 bg-white dark:bg-gray-900 shadow-xl transform transition-transform duration-300"
      :class="mobileSidebarOpen ? 'translate-x-0' : '-translate-x-full'"
      @click.stop
    >
      <div class="h-full flex flex-col">
        <!-- 移动端侧边栏头部 -->
        <div class="p-4 border-b border-gray-200 dark:border-gray-800 flex items-center justify-between">
          <div class="flex items-center space-x-3">
            <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center">
              <i class="pi pi-folder text-white"></i>
            </div>
            <h3 class="font-semibold text-gray-900 dark:text-white">文件管理</h3>
          </div>
          <button @click="closeMobileSidebar" class="p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800">
            <i class="pi pi-times text-gray-500"></i>
          </button>
        </div>

        <!-- 移动端导航内容（复用桌面端内容） -->
        <nav class="flex-1 p-4 overflow-y-auto">
          <div class="space-y-1">
            <div class="px-3 py-2">
              <h4 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">常用功能</h4>
              <div class="space-y-1">
                <router-link
                  to="/decompress"
                  @click="closeMobileSidebar"
                  class="flex items-center px-3 py-2 rounded-lg text-sm transition-colors"
                  :class="[
                    $route.path === '/decompress'
                      ? 'bg-primary/10 text-primary'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
                  ]"
                >
                  <i class="pi pi-file-import mr-3"></i>
                  文件解压
                </router-link>
                <router-link
                  to="/compress"
                  @click="closeMobileSidebar"
                  class="flex items-center px-3 py-2 rounded-lg text-sm transition-colors"
                  :class="[
                    $route.path === '/compress'
                      ? 'bg-primary/10 text-primary'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
                  ]"
                >
                  <i class="pi pi-compress mr-3"></i>
                  文件压缩
                </router-link>
                <button @click="closeMobileSidebar" class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                  <i class="pi pi-eye mr-3"></i>
                  文件预览
                </button>
              </div>
            </div>

            <div class="px-3 py-2">
              <h4 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">文件管理</h4>
              <div class="space-y-1">
                <button @click="closeMobileSidebar" class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                  <i class="pi pi-history mr-3"></i>
                  最近文件
                  <span class="ml-auto text-xs px-2 py-1 bg-gray-200 dark:bg-gray-700 rounded-full">12</span>
                </button>
                <button @click="closeMobileSidebar" class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                  <i class="pi pi-star mr-3"></i>
                  收藏夹
                  <span class="ml-auto text-xs px-2 py-1 bg-gray-200 dark:bg-gray-700 rounded-full">8</span>
                </button>
                <button @click="closeMobileSidebar" class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                  <i class="pi pi-trash mr-3"></i>
                  回收站
                </button>
              </div>
            </div>

            <div class="px-3 py-2">
              <h4 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">工具</h4>
              <div class="space-y-1">
                <button @click="closeMobileSidebar" class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                  <i class="pi pi-shield mr-3"></i>
                  安全检查
                </button>
                <button @click="closeMobileSidebar" class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                  <i class="pi pi-chart-bar mr-3"></i>
                  统计信息
                </button>
                <button @click="closeMobileSidebar" class="w-full flex items-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
                  <i class="pi pi-wrench mr-3"></i>
                  批量处理
                </button>
              </div>
            </div>
          </div>
        </nav>

        <!-- 移动端侧边栏底部 -->
        <div class="p-4 border-t border-gray-200 dark:border-gray-800">
          <div class="space-y-3">
            <div class="px-3">
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs text-gray-600 dark:text-gray-400">存储空间</span>
                <span class="text-xs font-medium text-gray-900 dark:text-white">23%</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                <div class="bg-primary h-1.5 rounded-full" style="width: 23%"></div>
              </div>
            </div>
            <button @click="closeMobileSidebar" class="w-full flex items-center justify-center px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors">
              <i class="pi pi-cog mr-2"></i>
              管理存储
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const mobileSidebarOpen = ref(false)

const openMobileSidebar = () => {
  mobileSidebarOpen.value = true
}

const closeMobileSidebar = () => {
  mobileSidebarOpen.value = false
}

// 暴露方法给父组件
defineExpose({
  openMobileSidebar,
  closeMobileSidebar
})
</script>

<style scoped>
/* 自定义滚动条样式 */
::-webkit-scrollbar {
  width: 4px;
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