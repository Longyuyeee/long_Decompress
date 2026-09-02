<template>
  <Teleport to="body">
  <Transition name="fade">
    <div v-if="visible" class="fixed inset-0 overflow-y-auto" :class="layerClass" role="dialog" aria-modal="true" :aria-labelledby="title ? modalTitleId : undefined">
      <div class="fixed inset-0 bg-black/60 transition-opacity" @click="handleBackdropClick"></div>

      <div class="flex min-h-full items-center justify-center p-4 text-center">
        <Transition name="pop">
          <div
            ref="modalContent"
            class="relative flex max-h-[min(78vh,42rem)] w-full transform flex-col overflow-hidden rounded-[1.4rem] bg-modal border border-subtle text-left shadow-2xl transition-all outline-none"
            :class="sizeClasses"
            tabindex="-1"
          >
            <button
              v-if="showCloseButton"
              @click="handleClose"
              :aria-label="cancelText || 'Close dialog'"
              class="absolute right-6 top-6 z-10 rounded-full p-2 hover:bg-input transition-all"
            >
              <i class="pi pi-times text-muted hover:text-content text-xs"></i>
            </button>

            <div v-if="title || $slots.title" class="shrink-0 px-6 pt-6 pb-3">
              <div class="flex items-center gap-4">
                <div v-if="icon" class="w-10 h-10 rounded-2xl bg-primary/10 flex items-center justify-center">
                   <i :class="[icon, 'text-primary']"></i>
                </div>
                <div>
                  <h3 :id="modalTitleId" class="text-lg font-black text-content tracking-tight leading-none mb-1">
                    <slot name="title">{{ title }}</slot>
                  </h3>
                  <p v-if="description" class="text-sm text-muted font-bold uppercase tracking-widest">
                    {{ description }}
                  </p>
                </div>
              </div>
            </div>

            <div class="min-h-0 overflow-y-auto px-6 py-4 text-content custom-scrollbar">
              <slot></slot>
            </div>

            <div v-if="showFooter || $slots.footer" class="shrink-0 px-6 pb-6 pt-2">
              <div class="flex items-center justify-end gap-3">
                <slot name="footer">
                  <button v-if="cancelText" @click="handleCancel" class="px-6 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-bold hover:text-content transition-all">
                    {{ cancelText }}
                  </button>
                   <button v-if="confirmText" @click="handleConfirm" :disabled="loading" class="px-6 py-2.5 rounded-xl bg-primary text-white text-xs font-black shadow-lg shadow-primary/20 hover:scale-105 active:scale-95 transition-all flex items-center gap-2 disabled:opacity-60 disabled:cursor-wait">
                    <i v-if="loading" class="pi pi-spin pi-spinner"></i>
                    {{ confirmText }}
                  </button>
                </slot>
              </div>
            </div>
          </div>
        </Transition>
      </div>
    </div>
  </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, useId, watch } from 'vue'

export interface Props {
  visible: boolean
  title?: string
  description?: string
  icon?: string
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'full'
  showCloseButton?: boolean
  showFooter?: boolean
  cancelText?: string
  confirmText?: string
  loading?: boolean
  closeOnBackdrop?: boolean
  closeOnEscape?: boolean
  layer?: 'default' | 'nested'
}

const props = withDefaults(defineProps<Props>(), {
  visible: false,
  size: 'md',
  showCloseButton: true,
  showFooter: false, 
  closeOnBackdrop: true,
  closeOnEscape: true,
  layer: 'default',
  loading: false
})

const emit = defineEmits(['update:visible', 'close', 'cancel', 'confirm'])
const modalContent = ref<HTMLElement | null>(null)
const modalTitleId = `modal-title-${useId()}`
let previouslyFocused: HTMLElement | null = null

// 打开模态时自动聚焦到内容区域
watch(() => props.visible, (isVisible) => {
  if (isVisible) {
    previouslyFocused = document.activeElement as HTMLElement | null
    void nextTick(() => modalContent.value?.focus())
  } else if (previouslyFocused) {
    const target = previouslyFocused
    previouslyFocused = null
    void nextTick(() => target.focus())
  }
})

const sizeClasses = computed(() => {
  const classes: Record<string, string> = {
    xs: 'max-w-[320px]',
    sm: 'max-w-[380px]',
    md: 'max-w-[460px] md:max-w-lg',
    lg: 'max-w-[min(46rem,calc(100vw-3rem))]',
    xl: 'max-w-[min(58rem,calc(100vw-3rem))]',
    full: 'max-w-full mx-4'
  }
  return classes[props.size]
})

onMounted(() => {
  if (!props.visible) return
  previouslyFocused = document.activeElement as HTMLElement | null
  void nextTick(() => modalContent.value?.focus())
})

const layerClass = computed(() => props.layer === 'nested' ? 'z-[400]' : 'z-[300]')

const handleClose = () => { emit('update:visible', false); emit('close'); }
const handleCancel = () => { emit('cancel'); handleClose(); }
const handleConfirm = () => { emit('confirm'); }
const handleBackdropClick = () => { if (props.closeOnBackdrop) handleClose(); }

const handleKeydown = (e: KeyboardEvent) => {
  if (props.closeOnEscape && e.key === 'Escape' && props.visible) handleClose();
  if (e.key !== 'Tab' || !props.visible || !modalContent.value) return

  const focusable = Array.from(modalContent.value.querySelectorAll<HTMLElement>(
    'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [href], [tabindex]:not([tabindex="-1"])'
  )).filter(element => !element.hasAttribute('aria-hidden'))
  if (focusable.length === 0) {
    e.preventDefault()
    modalContent.value.focus()
    return
  }

  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault()
    last.focus()
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault()
    first.focus()
  }
}

window.addEventListener('keydown', handleKeydown)
onUnmounted(() => window.removeEventListener('keydown', handleKeydown))

defineExpose({ close: handleClose, open: () => emit('update:visible', true) })
</script>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.9) translateY(20px); }
</style>
