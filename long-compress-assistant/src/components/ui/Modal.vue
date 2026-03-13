<template>
  <Transition name="fade">
    <div v-if="visible" class="fixed inset-0 z-50 overflow-y-auto" role="dialog" aria-modal="true">
      <div class="fixed inset-0 bg-black/60 transition-opacity" @click="handleBackdropClick"></div>

      <div class="flex min-h-full items-center justify-center p-4 text-center">
        <Transition name="pop">
          <div
            class="relative w-full transform overflow-hidden rounded-[2rem] bg-modal border border-subtle text-left shadow-2xl transition-all"
            :class="sizeClasses"
          >
            <button
              v-if="showCloseButton"
              @click="handleClose"
              class="absolute right-6 top-6 z-10 rounded-full p-2 hover:bg-input transition-all"
            >
              <i class="pi pi-times text-muted hover:text-content text-xs"></i>
            </button>

            <div v-if="title || $slots.title" class="px-8 pt-8 pb-4">
              <div class="flex items-center gap-4">
                <div v-if="icon" class="w-10 h-10 rounded-2xl bg-primary/10 flex items-center justify-center">
                   <i :class="[icon, 'text-primary']"></i>
                </div>
                <div>
                  <h3 class="text-lg font-black text-content tracking-tight leading-none mb-1">
                    <slot name="title">{{ title }}</slot>
                  </h3>
                  <p v-if="description" class="text-[10px] text-muted font-bold uppercase tracking-widest">
                    {{ description }}
                  </p>
                </div>
              </div>
            </div>

            <div class="px-8 py-6 text-content">
              <slot></slot>
            </div>

            <div v-if="showFooter || $slots.footer" class="px-8 pb-8">
              <div class="flex items-center justify-end gap-3">
                <slot name="footer">
                  <button v-if="cancelText" @click="handleCancel" class="px-6 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-bold hover:text-content transition-all">
                    {{ cancelText }}
                  </button>
                  <button v-if="confirmText" @click="handleConfirm" class="px-6 py-2.5 rounded-xl bg-primary text-white text-xs font-black shadow-lg shadow-primary/20 hover:scale-105 active:scale-95 transition-all flex items-center gap-2">
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
</template>

<script setup lang="ts">
import { computed, onUnmounted } from 'vue'

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
}

const props = withDefaults(defineProps<Props>(), {
  visible: false,
  size: 'md',
  showCloseButton: true,
  showFooter: false, 
  closeOnBackdrop: true,
  closeOnEscape: true,
  loading: false
})

const emit = defineEmits(['update:visible', 'close', 'cancel', 'confirm'])

const sizeClasses = computed(() => {
  const classes: Record<string, string> = {
    xs: 'max-w-[320px]',
    sm: 'max-w-[380px]',
    md: 'max-w-[460px] md:max-w-lg',
    lg: 'max-w-[92vw] md:max-w-2xl',
    xl: 'max-w-[95vw] md:max-w-4xl',
    full: 'max-w-full mx-4'
  }
  return classes[props.size]
})

const handleClose = () => { emit('update:visible', false); emit('close'); }
const handleCancel = () => { emit('cancel'); handleClose(); }
const handleConfirm = () => { emit('confirm'); }
const handleBackdropClick = () => { if (props.closeOnBackdrop) handleClose(); }

const handleKeydown = (e: KeyboardEvent) => {
  if (props.closeOnEscape && e.key === 'Escape' && props.visible) handleClose();
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
