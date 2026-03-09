<template>
  <!-- 模态框背景 -->
  <Transition name="fade">
    <div
      v-if="visible"
      class="fixed inset-0 z-50 overflow-y-auto"
      aria-labelledby="modal-title"
      role="dialog"
      aria-modal="true"
    >
      <!-- 背景遮罩 -->
      <div
        class="fixed inset-0 bg-black/50 transition-opacity"
        aria-hidden="true"
        @click="handleBackdropClick"
      ></div>

      <!-- 模态框内容 -->
      <div class="flex min-h-full items-center justify-center p-4 text-center">
        <Transition name="slide-up">
          <div
            class="relative w-full max-w-4xl transform overflow-hidden rounded-2xl glass-effect text-left shadow-xl transition-all"
            :class="sizeClasses"
          >
            <!-- 关闭按钮 -->
            <button
              v-if="showCloseButton"
              @click="handleClose"
              class="absolute right-4 top-4 z-10 rounded-full p-2 hover:bg-white/10 transition-colors focus:outline-none focus:ring-2 focus:ring-primary"
              aria-label="关闭"
            >
              <i class="pi pi-times text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"></i>
            </button>

            <!-- 标题区域 -->
            <div
              v-if="title || $slots.title"
              class="border-b border-gray-200 dark:border-gray-700 px-6 py-4"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                  <i v-if="icon" :class="icon" class="text-primary"></i>
                  <h3
                    id="modal-title"
                    class="text-lg font-semibold text-gray-900 dark:text-white"
                  >
                    <slot name="title">{{ title }}</slot>
                  </h3>
                </div>
                <div v-if="$slots['title-actions']">
                  <slot name="title-actions"></slot>
                </div>
              </div>
              <p
                v-if="description"
                class="mt-1 text-sm text-gray-600 dark:text-gray-400"
              >
                {{ description }}
              </p>
            </div>

            <!-- 内容区域 -->
            <div class="px-6 py-4">
              <slot></slot>
            </div>

            <!-- 底部操作区域 -->
            <div
              v-if="showFooter || $slots.footer"
              class="border-t border-gray-200 dark:border-gray-700 px-6 py-4"
            >
              <div class="flex items-center justify-between">
                <div v-if="$slots['footer-left']">
                  <slot name="footer-left"></slot>
                </div>
                <div class="flex items-center space-x-3">
                  <slot name="footer">
                    <button
                      v-if="cancelText"
                      @click="handleCancel"
                      class="glass-button px-4 py-2"
                      :disabled="loading"
                    >
                      {{ cancelText }}
                    </button>
                    <button
                      v-if="confirmText"
                      @click="handleConfirm"
                      class="glass-button-primary px-4 py-2"
                      :disabled="loading"
                      :class="{ 'opacity-50 cursor-not-allowed': loading }"
                    >
                      <i v-if="loading" class="pi pi-spin pi-spinner mr-2"></i>
                      {{ confirmText }}
                    </button>
                  </slot>
                </div>
              </div>
            </div>
          </div>
        </Transition>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed } from 'vue'

// 定义组件属�?
interface Props {
  visible: boolean
  title?: string
  description?: string
  icon?: string
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full'
  showCloseButton?: boolean
  showFooter?: boolean
  cancelText?: string
  confirmText?: string
  loading?: boolean
  closeOnBackdrop?: boolean
  closeOnEscape?: boolean
}

// 定义组件事件
interface Emits {
  (e: 'update:visible', value: boolean): void
  (e: 'close'): void
  (e: 'cancel'): void
  (e: 'confirm'): void
}

const props = withDefaults(defineProps<Props>(), {
  visible: false,
  size: 'md',
  showCloseButton: true,
  showFooter: true,
  closeOnBackdrop: true,
  closeOnEscape: true,
  loading: false
})

const emit = defineEmits<Emits>()

// 计算属�?
const sizeClasses = computed(() => {
  const classes: Record<string, string> = {
    sm: 'max-w-md',
    md: 'max-w-lg',
    lg: 'max-w-2xl',
    xl: 'max-w-4xl',
    full: 'max-w-full mx-4'
  }
  return classes[props.size]
})

// 方法
const handleClose = () => {
  emit('update:visible', false)
  emit('close')
}

const handleCancel = () => {
  emit('cancel')
  handleClose()
}

const handleConfirm = () => {
  emit('confirm')
}

const handleBackdropClick = () => {
  if (props.closeOnBackdrop) {
    handleClose()
  }
}

const handleKeydown = (event: KeyboardEvent) => {
  if (props.closeOnEscape && event.key === 'Escape' && props.visible) {
    handleClose()
  }
}

// 键盘事件监听
if (typeof window !== 'undefined') {
  window.addEventListener('keydown', handleKeydown)
}

// 清理事件监听
import { onUnmounted } from 'vue'
onUnmounted(() => {
  if (typeof window !== 'undefined') {
    window.removeEventListener('keydown', handleKeydown)
  }
})

// 暴露方法给父组件
defineExpose({
  close: handleClose,
  open: () => emit('update:visible', true)
})
</script>

<style scoped>
/* 淡入淡出动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* 上滑动画 */
.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.3s ease;
}

.slide-up-enter-from,
.slide-up-leave-to {
  opacity: 0;
  transform: translateY(20px);
}

/* 玻璃效果 */
.glass-effect {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.05) 100%
  );
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.dark .glass-effect {
  background: linear-gradient(
    135deg,
    rgba(0, 0, 0, 0.1) 0%,
    rgba(0, 0, 0, 0.05) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.1);
}

/* 滚动条样�?*/
.modal-content {
  max-height: calc(100vh - 200px);
}

.modal-content::-webkit-scrollbar {
  width: 6px;
}

.modal-content::-webkit-scrollbar-track {
  @apply bg-transparent;
}

.modal-content::-webkit-scrollbar-thumb {
  @apply bg-gray-300 dark:bg-gray-700 rounded-full;
}

.modal-content::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-400 dark:bg-gray-600;
}
</style>
