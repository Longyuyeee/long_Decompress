<template>
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6 sm:py-8">
    <!-- 页面标题 -->
    <div class="mb-6 sm:mb-8">
      <h1 class="text-2xl sm:text-3xl font-bold text-gray-900 dark:text-white">响应式设计测�?/h1>
      <p class="text-gray-600 dark:text-gray-400 text-sm sm:text-base mt-2">
        测试胧解压项目的响应式设计优化效果。调整浏览器窗口大小查看不同屏幕尺寸下的显示效果�?
      </p>
    </div>

    <!-- 断点指示�?-->
    <div class="mb-8 glass-card p-4">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">当前断点</h2>
      <div class="grid grid-cols-2 xs:grid-cols-3 sm:grid-cols-6 gap-3">
        <div class="text-center p-3 rounded-lg" :class="currentBreakpoint === 'xs' ? 'bg-primary/20 border border-primary' : 'bg-gray-100 dark:bg-gray-800'">
          <p class="font-medium text-gray-900 dark:text-white">XS</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">&lt; 475px</p>
          <div v-if="currentBreakpoint === 'xs'" class="mt-2">
            <i class="pi pi-check text-green-500"></i>
          </div>
        </div>
        <div class="text-center p-3 rounded-lg" :class="currentBreakpoint === 'sm' ? 'bg-primary/20 border border-primary' : 'bg-gray-100 dark:bg-gray-800'">
          <p class="font-medium text-gray-900 dark:text-white">SM</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">�?475px</p>
          <div v-if="currentBreakpoint === 'sm'" class="mt-2">
            <i class="pi pi-check text-green-500"></i>
          </div>
        </div>
        <div class="text-center p-3 rounded-lg" :class="currentBreakpoint === 'md' ? 'bg-primary/20 border border-primary' : 'bg-gray-100 dark:bg-gray-800'">
          <p class="font-medium text-gray-900 dark:text-white">MD</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">�?640px</p>
          <div v-if="currentBreakpoint === 'md'" class="mt-2">
            <i class="pi pi-check text-green-500"></i>
          </div>
        </div>
        <div class="text-center p-3 rounded-lg" :class="currentBreakpoint === 'lg' ? 'bg-primary/20 border border-primary' : 'bg-gray-100 dark:bg-gray-800'">
          <p class="font-medium text-gray-900 dark:text-white">LG</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">�?768px</p>
          <div v-if="currentBreakpoint === 'lg'" class="mt-2">
            <i class="pi pi-check text-green-500"></i>
          </div>
        </div>
        <div class="text-center p-3 rounded-lg" :class="currentBreakpoint === 'xl' ? 'bg-primary/20 border border-primary' : 'bg-gray-100 dark:bg-gray-800'">
          <p class="font-medium text-gray-900 dark:text-white">XL</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">�?1024px</p>
          <div v-if="currentBreakpoint === 'xl'" class="mt-2">
            <i class="pi pi-check text-green-500"></i>
          </div>
        </div>
        <div class="text-center p-3 rounded-lg" :class="currentBreakpoint === '2xl' ? 'bg-primary/20 border border-primary' : 'bg-gray-100 dark:bg-gray-800'">
          <p class="font-medium text-gray-900 dark:text-white">2XL</p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">�?1280px</p>
          <div v-if="currentBreakpoint === '2xl'" class="mt-2">
            <i class="pi pi-check text-green-500"></i>
          </div>
        </div>
      </div>
      <div class="mt-4 text-sm text-gray-600 dark:text-gray-400">
        <p>当前窗口宽度: <span class="font-medium">{{ windowWidth }}px</span></p>
        <p>检测到的断�? <span class="font-medium">{{ currentBreakpoint.toUpperCase() }}</span></p>
      </div>
    </div>

    <!-- 网格布局测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">响应式网格测�?/h2>

      <!-- 基础网格 -->
      <div class="mb-6">
        <h3 class="font-medium text-gray-700 dark:text-gray-300 mb-2">基础网格 (grid grid-cols-1 xs:grid-cols-2 sm:grid-cols-3)</h3>
        <div class="grid grid-cols-1 xs:grid-cols-2 sm:grid-cols-3 gap-3">
          <div v-for="i in 6" :key="i" class="p-4 rounded-lg bg-primary/10 border border-primary/20">
            <p class="font-medium text-gray-900 dark:text-white">项目 {{ i }}</p>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">响应式网格项�?/p>
          </div>
        </div>
      </div>

      <!-- 复杂网格 -->
      <div class="mb-6">
        <h3 class="font-medium text-gray-700 dark:text-gray-300 mb-2">复杂网格 (grid-responsive-2-3-4-5)</h3>
        <div class="grid-responsive-2-3-4-5 gap-2">
          <div v-for="i in 10" :key="i" class="p-3 rounded-lg bg-accent/10 border border-accent/20 text-center">
            <p class="text-sm font-medium text-gray-900 dark:text-white">项目 {{ i }}</p>
          </div>
        </div>
      </div>
    </div>

    <!-- 间距测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">响应式间距测�?/h2>

      <div class="space-y-responsive p-4 rounded-lg bg-gray-100 dark:bg-gray-800">
        <div class="p-3 rounded-lg bg-white dark:bg-gray-900">
          <p class="font-medium text-gray-900 dark:text-white">项目 1</p>
          <p class="text-sm text-gray-600 dark:text-gray-400">使用 space-y-responsive �?/p>
        </div>
        <div class="p-3 rounded-lg bg-white dark:bg-gray-900">
          <p class="font-medium text-gray-900 dark:text-white">项目 2</p>
          <p class="text-sm text-gray-600 dark:text-gray-400">垂直间距响应式变�?/p>
        </div>
        <div class="p-3 rounded-lg bg-white dark:bg-gray-900">
          <p class="font-medium text-gray-900 dark:text-white">项目 3</p>
          <p class="text-sm text-gray-600 dark:text-gray-400">XS: 16px, SM: 24px</p>
        </div>
      </div>
    </div>

    <!-- 文本测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">响应式文本测�?/h2>

      <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div class="p-4 rounded-lg bg-gray-100 dark:bg-gray-800">
          <p class="text-responsive-sm font-medium text-gray-900 dark:text-white mb-2">小文�?/p>
          <p class="text-xs text-gray-600 dark:text-gray-400">XS: 12px, SM: 14px, MD: 16px</p>
        </div>
        <div class="p-4 rounded-lg bg-gray-100 dark:bg-gray-800">
          <p class="text-responsive-base font-medium text-gray-900 dark:text-white mb-2">基础文本</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">XS: 14px, SM: 16px, MD: 18px</p>
        </div>
        <div class="p-4 rounded-lg bg-gray-100 dark:bg-gray-800">
          <p class="text-responsive-lg font-medium text-gray-900 dark:text-white mb-2">大文�?/p>
          <p class="text-xs text-gray-600 dark:text-gray-400">XS: 16px, SM: 18px, MD: 20px</p>
        </div>
      </div>
    </div>

    <!-- 触摸目标测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">触摸目标测试</h2>

      <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div class="text-center">
          <button class="touch-target p-2 rounded-lg bg-primary text-white font-medium">
            标准按钮
          </button>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-2">最�?44×44px</p>
        </div>
        <div class="text-center">
          <button class="touch-target-sm p-2 rounded-lg bg-accent text-white font-medium">
            小按�?
          </button>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-2">最�?36×36px</p>
        </div>
        <div class="text-center">
          <button class="touch-target-lg p-2 rounded-lg bg-success text-white font-medium">
            大按�?
          </button>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-2">最�?48×48px</p>
        </div>
      </div>
    </div>

    <!-- 显示/隐藏测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">响应式显�?隐藏测试</h2>

      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <div class="p-3 rounded-lg bg-gray-100 dark:bg-gray-800 text-center">
          <p class="font-medium text-gray-900 dark:text-white">XS 显示</p>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-1 show-xs-only">仅XS可见</p>
        </div>
        <div class="p-3 rounded-lg bg-gray-100 dark:bg-gray-800 text-center">
          <p class="font-medium text-gray-900 dark:text-white">SM 显示</p>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-1 show-sm-only">仅SM可见</p>
        </div>
        <div class="p-3 rounded-lg bg-gray-100 dark:bg-gray-800 text-center">
          <p class="font-medium text-gray-900 dark:text-white">MD 显示</p>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-1 show-md-only">仅MD可见</p>
        </div>
        <div class="p-3 rounded-lg bg-gray-100 dark:bg-gray-800 text-center">
          <p class="font-medium text-gray-900 dark:text-white">LG 显示</p>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-1 show-lg-only">仅LG可见</p>
        </div>
      </div>
    </div>

    <!-- 实际组件测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">实际组件响应式测�?/h2>

      <!-- 模拟文件统计 -->
      <div class="glass-card p-4 mb-4">
        <h3 class="font-medium text-gray-900 dark:text-white mb-3">文件统计信息</h3>
        <div class="grid grid-cols-1 xs:grid-cols-2 gap-2 xs:gap-3">
          <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
            <div class="flex items-center">
              <i class="pi pi-file text-gray-500 mr-3"></i>
              <div class="flex-1">
                <p class="font-medium text-gray-900 dark:text-white">文件总数</p>
                <p class="text-2xl font-bold text-primary">12</p>
              </div>
            </div>
          </div>
          <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
            <div class="flex items-center">
              <i class="pi pi-database text-gray-500 mr-3"></i>
              <div class="flex-1">
                <p class="font-medium text-gray-900 dark:text-white">总大�?/p>
                <p class="text-2xl font-bold text-primary">245 MB</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 模拟格式支持 -->
      <div class="glass-card p-4">
        <h3 class="font-medium text-gray-900 dark:text-white mb-3">支持格式</h3>
        <div class="grid grid-cols-2 xs:grid-cols-3 gap-1 xs:gap-2">
          <div class="text-center p-2 rounded-lg bg-primary/10">
            <i class="pi pi-file text-primary text-lg block mb-1"></i>
            <span class="text-xs text-gray-700 dark:text-gray-300">ZIP</span>
          </div>
          <div class="text-center p-2 rounded-lg bg-accent/10">
            <i class="pi pi-file text-accent text-lg block mb-1"></i>
            <span class="text-xs text-gray-700 dark:text-gray-300">RAR</span>
          </div>
          <div class="text-center p-2 rounded-lg bg-green-500/10">
            <i class="pi pi-file text-green-500 text-lg block mb-1"></i>
            <span class="text-xs text-gray-700 dark:text-gray-300">7Z</span>
          </div>
          <div class="text-center p-2 rounded-lg bg-purple-500/10">
            <i class="pi pi-file text-purple-500 text-lg block mb-1"></i>
            <span class="text-xs text-gray-700 dark:text-gray-300">TAR</span>
          </div>
          <div class="text-center p-2 rounded-lg bg-yellow-500/10">
            <i class="pi pi-file text-yellow-500 text-lg block mb-1"></i>
            <span class="text-xs text-gray-700 dark:text-gray-300">GZ</span>
          </div>
          <div class="text-center p-2 rounded-lg bg-red-500/10">
            <i class="pi pi-file text-red-500 text-lg block mb-1"></i>
            <span class="text-xs text-gray-700 dark:text-gray-300">BZ2</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 测试说明 -->
    <div class="glass-card p-4">
      <h3 class="font-medium text-gray-900 dark:text-white mb-2">测试说明</h3>
      <ul class="space-y-2 text-sm text-gray-600 dark:text-gray-400">
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>调整浏览器窗口大小，观察组件的响应式变化</span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>在移动设备上测试触摸目标尺寸是否足够</span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>检查文本在不同屏幕尺寸下的可读�?/span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>验证网格布局在不同断点下的适应�?/span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>测试响应式工具类的实际效�?/span>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

// 响应式状�?
const windowWidth = ref(window.innerWidth)
const currentBreakpoint = ref('xs')

// 计算当前断点
const calculateBreakpoint = () => {
  const width = window.innerWidth
  if (width >= 1536) return '2xl'
  if (width >= 1280) return 'xl'
  if (width >= 1024) return 'lg'
  if (width >= 768) return 'md'
  if (width >= 640) return 'sm'
  if (width >= 475) return 'xs'
  return 'xs' // 小于475px也归为xs
}

// 更新窗口尺寸和断�?
const updateWindowSize = () => {
  windowWidth.value = window.innerWidth
  currentBreakpoint.value = calculateBreakpoint()
}

// 生命周期
onMounted(() => {
  updateWindowSize()
  window.addEventListener('resize', updateWindowSize)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateWindowSize)
})
</script>

<style scoped>
.glass-card {
  @apply bg-white/10 dark:bg-black/10 backdrop-blur-sm border border-gray-300 dark:border-gray-600 rounded-xl;
}
</style>
