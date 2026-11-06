<template>
  <Teleport to="body">
  <Transition name="fade" :css="active">
    <div v-if="visible && active" class="dialog-overlay fixed inset-0 overflow-hidden" :class="layerClass" role="dialog" aria-modal="true" :aria-labelledby="title ? modalTitleId : undefined">
      <div class="fixed inset-0 bg-black/60 transition-opacity" @click="handleBackdropClick"></div>

      <div class="dialog-viewport flex h-full items-center justify-center text-center">
        <Transition name="pop">
          <div
            ref="modalContent"
            class="app-dialog modal-no-glass relative flex w-full transform flex-col overflow-hidden rounded-[1.4rem] bg-modal border border-subtle text-left shadow-2xl transition-all outline-none"
            :style="{ '--dialog-width': sizeWidth }"
            tabindex="-1"
          >
            <button
              v-if="showCloseButton"
              @click="handleClose"
              :aria-label="cancelText || 'Close dialog'"
              class="dialog-close absolute z-10 rounded-full hover:bg-input transition-all"
            >
              <i class="pi pi-times text-muted hover:text-content text-xs"></i>
            </button>

            <div v-if="title || $slots.title" class="dialog-header shrink-0">
              <div class="dialog-heading flex items-center">
                <div v-if="icon" class="dialog-icon rounded-2xl bg-primary/10 flex items-center justify-center">
                   <i :class="[icon, 'text-primary']"></i>
                </div>
                <div class="min-w-0 flex-1"><div class="dialog-title-copy">
                  <h3 :id="modalTitleId" class="text-lg font-black text-content tracking-tight leading-none mb-1">
                    <slot name="title">{{ title }}</slot>
                  </h3>
                  <HelpHint v-if="description && descriptionAsHelp" :label="(title || '弹窗') + '说明'">
                    {{ description }}
                  </HelpHint>
                </div><p v-if="description && !descriptionAsHelp" class="mt-1 text-xs text-muted leading-relaxed">{{description}}</p></div>
              </div>
            </div>

            <div class="dialog-body min-h-0 text-content custom-scrollbar">
              <slot></slot>
            </div>

            <div v-if="showFooter || $slots.footer" class="dialog-footer shrink-0">
              <div class="dialog-actions flex items-center justify-end">
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
import HelpHint from './HelpHint.vue'
import { computed, nextTick, onMounted, onUnmounted, onActivated, onDeactivated, ref, useId, watch } from 'vue'

export interface Props {
  visible: boolean
  title?: string
  description?: string
  descriptionAsHelp?: boolean
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
const active = ref(true)
onActivated(() => { active.value = true })
onDeactivated(() => { active.value = false })
let previouslyFocused: HTMLElement | null = null

// 打开模态时自动聚焦到内容区域
watch(() => props.visible && active.value, (isVisible) => {
  if (isVisible) {
    previouslyFocused = document.activeElement as HTMLElement | null
    void nextTick(() => modalContent.value?.focus())
  } else if (previouslyFocused) {
    const target = previouslyFocused
    previouslyFocused = null
    if (active.value) void nextTick(() => target.focus())
  }
})

const sizeWidth = computed(() => {
  const widths: Record<string, string> = {
    xs: '20rem',
    sm: '24rem',
    md: '32rem',
    lg: '46rem',
    xl: '58rem',
    full: '100%'
  }
  return widths[props.size]
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
  if (!props.visible || !active.value || e.defaultPrevented) return
  const ownDialog = modalContent.value?.closest('[role="dialog"]')
  const dialogs = Array.from(document.querySelectorAll<HTMLElement>('[role="dialog"][aria-modal="true"]')).filter(dialog => dialog.getClientRects().length)
  const topDialog = dialogs.reduce<HTMLElement | undefined>((top, dialog) => !top || Number(getComputedStyle(dialog).zIndex) >= Number(getComputedStyle(top).zIndex) ? dialog : top, undefined)
  if (topDialog && ownDialog !== topDialog) return
  if (props.closeOnEscape && e.key === 'Escape' && props.visible) handleClose();
  if (e.key !== 'Tab' || !props.visible || !modalContent.value) return

  const focusable = Array.from(modalContent.value.querySelectorAll<HTMLElement>(
    'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), summary, [href], [tabindex]:not([tabindex="-1"])'
  )).filter(element => !element.hasAttribute('aria-hidden'))
  if (focusable.length === 0) {
    e.preventDefault()
    modalContent.value.focus()
    return
  }

  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  if (e.shiftKey && (document.activeElement === first || document.activeElement === modalContent.value)) {
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
.dialog-overlay { --dialog-gutter: clamp(8px, 2vmin, 16px); }
.dialog-viewport { min-width: 0; padding: var(--dialog-gutter); }
.app-dialog {
  min-width: 0;
  min-height: 0;
  max-width: min(var(--dialog-width), 100%);
  max-height: min(88vh, 48rem, calc(100vh - 2 * var(--dialog-gutter)));
  max-height: min(88dvh, 48rem, calc(100dvh - 2 * var(--dialog-gutter)));
}
.dialog-header { padding: 16px 20px 10px; }
.dialog-heading { min-width: 0; gap: clamp(8px, 1rem, 16px); }
.dialog-icon { width: 40px; height: 40px; flex-shrink: 0; }
.dialog-close { top: 12px; right: 12px; width: 32px; height: 32px; display: grid; place-items: center; }
.dialog-close i { font-size: clamp(12px, .75rem, 20px); }
.dialog-title-copy { display: flex; align-items: center; gap: 8px; min-width: 0; padding-right: 32px; }
.dialog-title-copy h3 { margin: 0; font-size: clamp(18px, 1.125rem, 28px); line-height: 1.4; overflow-wrap: anywhere; }
.dialog-header p { font-size: clamp(12px, .75rem, 16px); overflow-wrap: anywhere; }
/* Only the content scrolls: the close button and action footer stay reachable. */
.dialog-body { flex: 1 1 auto; min-width: 0; overflow-y: auto; overflow-x: hidden; overscroll-behavior: contain; padding: 12px 20px; overflow-wrap: anywhere; }
.dialog-body::-webkit-scrollbar { width: 6px; }
.dialog-body::-webkit-scrollbar-thumb { border-radius: 6px; background: var(--text-muted); }
.dialog-body::-webkit-scrollbar-track { background: transparent; }
.dialog-body > :deep(*) { min-width: 0; max-width: 100%; }
.dialog-footer { min-width: 0; padding: 10px 20px 16px; }
.dialog-actions { min-width: 0; flex-wrap: wrap; gap: clamp(8px, .75rem, 12px); }
.dialog-actions > :deep(button) { min-width: 0; max-width: 100%; overflow-wrap: anywhere; }
.dialog-actions > button { padding: 10px clamp(12px, 1.5rem, 24px); font-size: clamp(12px, .75rem, 16px); }
@media (max-height: 420px), (max-width: 420px) {
  .dialog-header { padding: 12px 16px 8px; }
  .dialog-body { padding: 10px 16px; }
  .dialog-footer { padding: 8px 16px 12px; }
}

.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.9) translateY(20px); }
</style>
