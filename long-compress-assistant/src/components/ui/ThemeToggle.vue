<template>
  <div class="theme-toggle" :class="[sizeClass, variantClass]">
    <!-- 图标按钮变体 -->
    <button
      v-if="variant === 'icon'"
      @click="toggleTheme"
      class="theme-toggle-icon"
      :class="{ 'theme-toggle-icon-active': isDark }"
      :title="`切换主题 (当前: ${currentThemeLabel})`"
      aria-label="切换主题"
    >
      <i :class="currentThemeIcon" class="theme-toggle-icon-inner"></i>
    </button>

    <!-- 开关变�?-->
    <div v-else-if="variant === 'switch'" class="theme-toggle-switch">
      <button
        @click="toggleTheme"
        class="theme-toggle-switch-button"
        :class="{ 'theme-toggle-switch-button-dark': isDark }"
        :title="`切换主题 (当前: ${currentThemeLabel})`"
        aria-label="切换主题"
      >
        <span class="theme-toggle-switch-track">
          <i class="pi pi-sun theme-toggle-switch-icon-light"></i>
          <i class="pi pi-moon theme-toggle-switch-icon-dark"></i>
        </span>
        <span class="theme-toggle-switch-thumb" :class="{ 'theme-toggle-switch-thumb-dark': isDark }"></span>
      </button>
      <span v-if="showLabel" class="theme-toggle-switch-label">
        {{ currentThemeLabel }}
      </span>
    </div>

    <!-- 按钮变体（默认） -->
    <button
      v-else
      @click="toggleTheme"
      class="theme-toggle-button"
      :class="{ 'theme-toggle-button-active': isDark }"
      :title="`切换主题 (当前: ${currentThemeLabel})`"
      aria-label="切换主题"
    >
      <i v-if="showIcon" :class="currentThemeIcon" class="theme-toggle-button-icon"></i>
      <span v-if="showLabel" class="theme-toggle-button-label">
        {{ currentThemeLabel }}
      </span>
    </button>

    <!-- 主题选择菜单 -->
    <div v-if="showMenu" class="theme-toggle-menu">
      <div class="theme-toggle-menu-header">
        <h4 class="theme-toggle-menu-title">选择主题</h4>
        <button @click="showMenu = false" class="theme-toggle-menu-close" aria-label="关闭菜单">
          <i class="pi pi-times"></i>
        </button>
      </div>
      <div class="theme-toggle-menu-options">
        <button
          v-for="option in themeOptions"
          :key="option.value"
          @click="setTheme(option.value as any)"
          class="theme-toggle-menu-option"
          :class="{ 'theme-toggle-menu-option-active': isOptionActive(option.value) }"
        >
          <i :class="option.icon" class="theme-toggle-menu-option-icon"></i>
          <span class="theme-toggle-menu-option-label">{{ option.label }}</span>
          <i
            v-if="isOptionActive(option.value)"
            class="pi pi-check theme-toggle-menu-option-check"
          ></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useTheme, type ThemeToggleProps } from '@/composables/useTheme'

const props = withDefaults(defineProps<ThemeToggleProps>(), {
  showLabel: true,
  showIcon: true,
  size: 'md',
  variant: 'button'
})

const emit = defineEmits<{
  toggle: [isDark: boolean]
  change: [theme: 'light' | 'dark' | 'auto']
}>()

const showMenu = ref(false)
const { isDark, currentThemeLabel, currentThemeIcon, themeOptions, toggleTheme, setTheme } = useTheme()

// 计算类名
const sizeClass = computed(() => `theme-toggle-${props.size}`)
const variantClass = computed(() => `theme-toggle-${props.variant}`)

// 检查选项是否激�?
const isOptionActive = (optionValue: string) => {
  const savedTheme = localStorage.getItem('dark-mode')

  if (optionValue === 'auto') {
    return savedTheme === null
  }

  if (optionValue === 'light') {
    return savedTheme === 'false'
  }

  if (optionValue === 'dark') {
    return savedTheme === 'true'
  }

  return false
}

// 处理主题切换
const handleToggleTheme = () => {
  if (props.variant === 'switch' || props.variant === 'icon') {
    toggleTheme()
    emit('toggle', isDark.value)
  } else {
    showMenu.value = !showMenu.value
  }
}

// 处理主题设置
const handleSetTheme = (theme: 'light' | 'dark' | 'auto') => {
  setTheme(theme)
  showMenu.value = false
  emit('change', theme)
  emit('toggle', isDark.value)
}

// 点击外部关闭菜单
const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement
  if (!target.closest('.theme-toggle')) {
    showMenu.value = false
  }
}

// 添加全局点击监听
if (typeof window !== 'undefined') {
  window.addEventListener('click', handleClickOutside)
}

// 清理事件监听
import { onUnmounted } from 'vue'
onUnmounted(() => {
  if (typeof window !== 'undefined') {
    window.removeEventListener('click', handleClickOutside)
  }
})
</script>

<style scoped>
.theme-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* 图标按钮变体 */
.theme-toggle-icon {
  @apply w-10 h-10 rounded-full flex items-center justify-center transition-all duration-300;
  @apply bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400;
  @apply hover:bg-gray-200 dark:hover:bg-gray-700 hover:text-gray-800 dark:hover:text-gray-200;
  @apply focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 dark:focus:ring-offset-gray-900;
}

.theme-toggle-icon-active {
  @apply bg-primary/10 text-primary;
}

.theme-toggle-icon-inner {
  @apply text-lg;
}

/* 开关变�?*/
.theme-toggle-switch {
  @apply flex items-center space-x-3;
}

.theme-toggle-switch-button {
  @apply relative inline-flex h-6 w-12 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 dark:focus:ring-offset-gray-900;
  @apply bg-gray-200 dark:bg-gray-700;
}

.theme-toggle-switch-button-dark {
  @apply bg-primary;
}

.theme-toggle-switch-track {
  @apply absolute inset-0 flex items-center justify-between px-1;
}

.theme-toggle-switch-icon-light,
.theme-toggle-switch-icon-dark {
  @apply text-xs;
}

.theme-toggle-switch-icon-light {
  @apply text-yellow-500;
}

.theme-toggle-switch-icon-dark {
  @apply text-blue-400;
}

.theme-toggle-switch-thumb {
  @apply pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-lg ring-0 transition-transform;
  @apply translate-x-0.5;
}

.theme-toggle-switch-thumb-dark {
  @apply translate-x-6;
}

.theme-toggle-switch-label {
  @apply text-sm font-medium text-gray-700 dark:text-gray-300;
}

/* 按钮变体 */
.theme-toggle-button {
  @apply flex items-center space-x-2 px-4 py-2 rounded-lg transition-all duration-300;
  @apply bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300;
  @apply hover:bg-gray-200 dark:hover:bg-gray-700;
  @apply focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 dark:focus:ring-offset-gray-900;
}

.theme-toggle-button-active {
  @apply bg-primary/10 text-primary;
}

.theme-toggle-button-icon {
  @apply text-lg;
}

.theme-toggle-button-label {
  @apply text-sm font-medium;
}

/* 主题选择菜单 */
.theme-toggle-menu {
  @apply absolute top-full right-0 mt-2 w-48 rounded-lg shadow-lg z-50;
  @apply bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700;
  @apply animate-slide-down;
}

.theme-toggle-menu-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700;
}

.theme-toggle-menu-title {
  @apply font-semibold text-gray-900 dark:text-white;
}

.theme-toggle-menu-close {
  @apply p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors;
  @apply text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300;
}

.theme-toggle-menu-options {
  @apply py-2;
}

.theme-toggle-menu-option {
  @apply w-full flex items-center justify-between px-4 py-3 text-left transition-colors;
  @apply hover:bg-gray-100 dark:hover:bg-gray-700;
  @apply text-gray-700 dark:text-gray-300;
}

.theme-toggle-menu-option-active {
  @apply bg-primary/10 text-primary;
}

.theme-toggle-menu-option-icon {
  @apply mr-3 text-lg;
}

.theme-toggle-menu-option-label {
  @apply flex-1 font-medium;
}

.theme-toggle-menu-option-check {
  @apply text-primary;
}

/* 尺寸变体 */
.theme-toggle-sm .theme-toggle-button {
  @apply px-3 py-1.5 text-sm;
}

.theme-toggle-sm .theme-toggle-icon {
  @apply w-8 h-8;
}

.theme-toggle-sm .theme-toggle-switch-button {
  @apply h-5 w-10;
}

.theme-toggle-sm .theme-toggle-switch-thumb {
  @apply h-4 w-4;
}

.theme-toggle-sm .theme-toggle-switch-thumb-dark {
  @apply translate-x-5;
}

.theme-toggle-lg .theme-toggle-button {
  @apply px-6 py-3 text-lg;
}

.theme-toggle-lg .theme-toggle-icon {
  @apply w-12 h-12;
}

.theme-toggle-lg .theme-toggle-switch-button {
  @apply h-8 w-16;
}

.theme-toggle-lg .theme-toggle-switch-thumb {
  @apply h-7 w-7;
}

.theme-toggle-lg .theme-toggle-switch-thumb-dark {
  @apply translate-x-8;
}

/* 动画 */
@keyframes slide-down {
  0% {
    opacity: 0;
    transform: translateY(-10px);
  }
  100% {
    opacity: 1;
    transform: translateY(0);
  }
}

.animate-slide-down {
  animation: slide-down 0.2s ease-out;
}
</style>
