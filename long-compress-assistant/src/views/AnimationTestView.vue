<template>
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6 sm:py-8">
    <!-- 页面标题 -->
    <div class="mb-6 sm:mb-8">
      <h1 class="text-2xl sm:text-3xl font-bold text-gray-900 dark:text-white">动画效果测试</h1>
      <p class="text-gray-600 dark:text-gray-400 text-sm sm:text-base mt-2">
        测试胧解压项目的动画效果优化。包含页面过渡、组件交互、加载状态和微交互动画�?
      </p>
    </div>

    <!-- 动画控制面板 -->
    <div class="mb-8 glass-card p-4">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">动画控制</h2>
      <div class="flex flex-wrap gap-3">
        <button @click="toggleReducedMotion" class="glass-button px-4 py-2">
          {{ reducedMotion ? '启用动画' : '减少动画' }}
        </button>
        <button @click="resetAllAnimations" class="glass-button px-4 py-2">
          重置所有动�?
        </button>
        <button @click="playAllAnimations" class="glass-button-primary px-4 py-2">
          播放所有动�?
        </button>
      </div>
      <div class="mt-4 text-sm text-gray-600 dark:text-gray-400">
        <p>当前状�? <span class="font-medium">{{ reducedMotion ? '减少动画模式' : '正常动画模式' }}</span></p>
        <p class="mt-1">提示: 点击各个动画卡片可以触发对应的动画效�?/p>
      </div>
    </div>

    <!-- 页面过渡动画测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">页面过渡动画</h2>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- 页面进入动画 -->
        <div class="glass-card p-4 cursor-pointer" @click="triggerAnimation('pageEnter')" :class="{ 'page-transition-enter-active': activeAnimations.pageEnter }">
          <h3 class="font-medium text-gray-900 dark:text-white mb-2">页面进入动画</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">模拟页面加载时的平滑进入效果</p>
          <div class="h-32 rounded-lg bg-gradient-to-br from-primary/20 to-accent/20 flex items-center justify-center">
            <div class="text-center">
              <i class="pi pi-arrow-right text-primary text-2xl mb-2"></i>
              <p class="text-sm font-medium">点击测试</p>
            </div>
          </div>
        </div>

        <!-- 模态框动画 -->
        <div class="glass-card p-4 cursor-pointer" @click="triggerAnimation('modalEnter')" :class="{ 'modal-transition-enter-active': activeAnimations.modalEnter }">
          <h3 class="font-medium text-gray-900 dark:text-white mb-2">模态框动画</h3>
          <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">对话框和模态框的弹出效�?/p>
          <div class="h-32 rounded-lg bg-gradient-to-br from-purple-500/20 to-pink-500/20 flex items-center justify-center">
            <div class="text-center">
              <i class="pi pi-window-maximize text-purple-500 text-2xl mb-2"></i>
              <p class="text-sm font-medium">点击测试</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 组件交互动画测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">组件交互动画</h2>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- 按钮交互 -->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">按钮交互</h3>
          <button class="w-full button-interaction glass-button-primary py-3">
            悬停/点击测试
          </button>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">悬停时上浮，点击时下�?/p>
        </div>

        <!-- 卡片悬停 -->
        <div class="glass-card p-4 card-hover-lift cursor-pointer">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">卡片悬停</h3>
          <div class="h-20 rounded-lg bg-gradient-to-br from-green-500/20 to-blue-500/20 flex items-center justify-center">
            <i class="pi pi-card text-green-500 text-2xl"></i>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">悬停时上浮并增加阴影</p>
        </div>

        <!-- 图标交互 -->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">图标交互</h3>
          <div class="flex justify-center">
            <button class="icon-interaction p-3 rounded-full bg-primary/10 hover:bg-primary/20">
              <i class="pi pi-star text-primary text-2xl"></i>
            </button>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">悬停时轻微放�?/p>
        </div>

        <!-- 输入框焦�?-->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">输入框焦�?/h3>
          <input type="text" class="w-full input-focus-animation glass-input" placeholder="点击聚焦测试">
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">聚焦时轻微放大并增加阴影</p>
        </div>
      </div>
    </div>

    <!-- 加载状态动画测�?-->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">加载状态动�?/h2>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        <!-- 骨架�?-->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">骨架屏加�?/h3>
          <div class="space-y-3">
            <div class="skeleton-loading h-4 rounded"></div>
            <div class="skeleton-loading h-4 rounded w-3/4"></div>
            <div class="skeleton-loading h-20 rounded"></div>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">内容加载时的占位动画</p>
        </div>

        <!-- 进度条加�?-->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">进度条加�?/h3>
          <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
            <div class="progress-loading h-2 rounded-full" style="width: 65%"></div>
          </div>
          <div class="mt-4 flex justify-center">
            <div class="spinner-smooth w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">进度条和旋转加载�?/p>
        </div>

        <!-- 脉冲加载 -->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">脉冲加载</h3>
          <div class="flex justify-center items-center h-24">
            <div class="pulse-gentle w-16 h-16 rounded-full bg-primary/20 flex items-center justify-center">
              <i class="pi pi-cloud-download text-primary text-2xl"></i>
            </div>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">轻柔的脉冲效�?/p>
        </div>
      </div>
    </div>

    <!-- 微交互动画测�?-->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">微交互动�?/h2>

      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- 成功反馈 -->
        <div class="glass-card p-4 cursor-pointer" @click="triggerAnimation('success')" :class="{ 'success-feedback': activeAnimations.success }">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">成功反馈</h3>
          <div class="h-16 rounded-lg bg-green-500/10 flex items-center justify-center">
            <i class="pi pi-check-circle text-green-500 text-2xl"></i>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">操作成功时的弹跳反馈</p>
        </div>

        <!-- 错误反馈 -->
        <div class="glass-card p-4 cursor-pointer" @click="triggerAnimation('error')" :class="{ 'error-shake': activeAnimations.error }">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">错误反馈</h3>
          <div class="h-16 rounded-lg bg-red-500/10 flex items-center justify-center">
            <i class="pi pi-times-circle text-red-500 text-2xl"></i>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">操作错误时的抖动反馈</p>
        </div>

        <!-- 通知动画 -->
        <div class="glass-card p-4 cursor-pointer" @click="triggerAnimation('notification')" :class="{ 'notification-slide': activeAnimations.notification }">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">通知动画</h3>
          <div class="h-16 rounded-lg bg-blue-500/10 flex items-center justify-center">
            <i class="pi pi-bell text-blue-500 text-2xl"></i>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">通知消息的滑入效�?/p>
        </div>

        <!-- 列表项动�?-->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">列表项动�?/h3>
          <div class="space-y-2">
            <div v-for="item in listItems" :key="item.id"
                 class="p-2 rounded bg-gray-100 dark:bg-gray-800 flex items-center justify-between"
                 :class="{ 'list-item-add': item.justAdded, 'list-item-remove': item.removing }">
              <span>项目 {{ item.id }}</span>
              <button @click="removeListItem(item.id)" class="text-gray-400 hover:text-red-500">
                <i class="pi pi-times"></i>
              </button>
            </div>
          </div>
          <button @click="addListItem" class="w-full glass-button mt-3 py-2">
            添加项目
          </button>
        </div>
      </div>
    </div>

    <!-- 高级动画效果测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">高级动画效果</h2>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- 毛玻璃效�?-->
        <div class="glass-card p-4 glass-morph cursor-pointer">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">毛玻璃动�?/h3>
          <div class="h-32 rounded-lg bg-gradient-to-br from-white/10 to-transparent backdrop-blur-sm flex items-center justify-center">
            <i class="pi pi-snowflake text-white text-3xl"></i>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">悬停时增强毛玻璃效果</p>
        </div>

        <!-- 渐变背景 -->
        <div class="glass-card p-4 gradient-shift cursor-pointer">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">渐变背景动画</h3>
          <div class="h-32 rounded-lg flex items-center justify-center">
            <i class="pi pi-palette text-white text-3xl"></i>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">渐变背景的流动效�?/p>
        </div>

        <!-- 3D翻转 -->
        <div class="glass-card p-4 flip-3d cursor-pointer">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">3D翻转效果</h3>
          <div class="h-32 rounded-lg bg-gradient-to-br from-orange-500/20 to-yellow-500/20 flex items-center justify-center">
            <i class="pi pi-cube text-orange-500 text-3xl"></i>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">悬停时的3D透视效果</p>
        </div>

        <!-- 波纹效果 -->
        <div class="glass-card p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-3">波纹点击效果</h3>
          <button class="w-full ripple-effect glass-button-primary py-3">
            点击测试波纹效果
          </button>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">点击时的水波纹扩散效�?/p>
        </div>
      </div>
    </div>

    <!-- 性能优化测试 -->
    <div class="mb-8">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">性能优化</h2>

      <div class="glass-card p-4">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <h3 class="font-medium text-gray-900 dark:text-white mb-2">硬件加�?/h3>
            <div class="hardware-accelerated p-4 rounded-lg bg-primary/10">
              <p class="text-sm">启用硬件加速的动画元素</p>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">使用 transform: translateZ(0) 优化性能</p>
            </div>
          </div>
          <div>
            <h3 class="font-medium text-gray-900 dark:text-white mb-2">减少运动偏好</h3>
            <div class="animation-optimized p-4 rounded-lg bg-green-500/10">
              <p class="text-sm">尊重用户偏好设置</p>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">支持 prefers-reduced-motion 媒体查询</p>
            </div>
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
          <span>点击各个动画卡片可以触发对应的动画效�?/span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>悬停在交互元素上查看悬停动画效果</span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>使用"减少动画"按钮模拟用户偏好设置</span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>测试不同设备上的动画性能表现</span>
        </li>
        <li class="flex items-start">
          <i class="pi pi-check-circle text-green-500 mt-0.5 mr-2"></i>
          <span>验证动画是否增强用户体验而不干扰操作</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'

// 状态管�?
const reducedMotion = ref(false)
const activeAnimations = reactive({
  pageEnter: false,
  modalEnter: false,
  success: false,
  error: false,
  notification: false
})

// 列表项管�?
const listItems = ref([
  { id: 1, justAdded: false, removing: false },
  { id: 2, justAdded: false, removing: false },
  { id: 3, justAdded: false, removing: false }
])

let nextItemId = 4

// 方法：触发动�?
const triggerAnimation = (animationName: keyof typeof activeAnimations) => {
  if (reducedMotion.value) return

  activeAnimations[animationName] = true
  setTimeout(() => {
    activeAnimations[animationName] = false
  }, 1000) // 动画持续时间后重�?
}

// 方法：切换减少动画模�?
const toggleReducedMotion = () => {
  reducedMotion.value = !reducedMotion.value

  // 应用减少动画模式
  if (reducedMotion.value) {
    document.documentElement.style.setProperty('--animation-duration', '0.01ms')
  } else {
    document.documentElement.style.removeProperty('--animation-duration')
  }
}

// 方法：重置所有动�?
const resetAllAnimations = () => {
  Object.keys(activeAnimations).forEach(key => {
    activeAnimations[key as keyof typeof activeAnimations] = false
  })
}

// 方法：播放所有动�?
const playAllAnimations = () => {
  if (reducedMotion.value) return

  const animations = Object.keys(activeAnimations) as Array<keyof typeof activeAnimations>
  animations.forEach((anim, index) => {
    setTimeout(() => {
      triggerAnimation(anim)
    }, index * 300)
  })
}

// 方法：添加列表项
const addListItem = () => {
  const newItem = { id: nextItemId++, justAdded: true, removing: false }
  listItems.value.push(newItem)

  // 移除刚刚添加的标�?
  setTimeout(() => {
    const item = listItems.value.find(item => item.id === newItem.id)
    if (item) item.justAdded = false
  }, 300)
}

// 方法：移除列表项
const removeListItem = (id: number) => {
  const itemIndex = listItems.value.findIndex(item => item.id === id)
  if (itemIndex !== -1) {
    listItems.value[itemIndex].removing = true

    // 动画完成后移�?
    setTimeout(() => {
      listItems.value.splice(itemIndex, 1)
    }, 300)
  }
}
</script>

<style scoped>
.glass-card {
  @apply bg-white/10 dark:bg-black/10 backdrop-blur-sm border border-gray-300 dark:border-gray-600 rounded-xl;
}

.glass-button {
  @apply px-4 py-2 bg-white/10 dark:bg-black/10 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-white/20 dark:hover:bg-black/20 transition-all focus:outline-none focus:ring-2 focus:ring-primary;
}

.glass-button-primary {
  @apply px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-dark transition-all focus:outline-none focus:ring-2 focus:ring-primary;
}

.glass-input {
  @apply px-3 py-2 bg-white/10 dark:bg-black/10 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all;
}

/* 自定义CSS变量用于动画控制 */
:root {
  --animation-duration: 300ms;
}

/* 应用减少动画偏好 */
@media (prefers-reduced-motion: reduce) {
  .page-transition-enter-active,
  .modal-transition-enter-active,
  .button-interaction,
  .card-hover-lift,
  .icon-interaction,
  .input-focus-animation,
  .skeleton-loading,
  .progress-loading,
  .spinner-smooth,
  .pulse-gentle,
  .success-feedback,
  .error-shake,
  .notification-slide,
  .list-item-add,
  .list-item-remove,
  .glass-morph,
  .gradient-shift,
  .flip-3d {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
</style>
