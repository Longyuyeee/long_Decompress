<script setup lang="ts">
import { useAppStore } from '@/stores/app'

defineProps<{
  hasFinished: boolean
  activeCount: number
  pausableCount: number
  pausedCount: number
  pendingCount: number
  busy: boolean
}>()

defineEmits<{
  clearFinished: []
  cancelActive: []
  pauseActive: []
  resumePaused: []
  openSettings: []
  start: []
}>()

const appStore = useAppStore()
</script>

<template>
  <div
    data-testid="compression-top-actions"
    class="flex items-center gap-2 md:gap-3 shrink-0"
  >
    <button
      v-if="hasFinished"
      type="button"
      class="h-8 md:h-9 px-3 rounded-lg bg-input border border-subtle text-muted text-xs font-bold hover:text-content hover:border-primary transition-all flex items-center gap-2"
      @click="$emit('clearFinished')"
    >
      <i class="pi pi-trash text-xs"></i>
      <span class="hidden md:inline">{{ appStore.t('compress.clear_finished') }}</span>
    </button>
    <button
      v-if="pausableCount > 0"
      type="button"
      class="h-8 md:h-9 px-3 rounded-lg bg-amber-500/10 border border-amber-500/30 text-amber-400 text-xs font-bold hover:bg-amber-500/20 transition-all flex items-center gap-2"
      @click="$emit('pauseActive')"
    >
      <i class="pi pi-pause text-xs"></i>
      <span class="hidden md:inline">{{ appStore.t('tasks.pause_all') }}</span>
    </button>
    <button
      v-if="pausedCount > 0"
      type="button"
      class="h-8 md:h-9 px-3 rounded-lg bg-green-500/10 border border-green-500/30 text-green-400 text-xs font-bold hover:bg-green-500/20 transition-all flex items-center gap-2"
      @click="$emit('resumePaused')"
    >
      <i class="pi pi-play text-xs"></i>
      <span class="hidden md:inline">{{ appStore.t('tasks.resume_all') }}</span>
    </button>
    <button
      v-if="activeCount > 0"
      type="button"
      class="h-8 md:h-9 px-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-xs font-bold hover:bg-red-500/20 transition-all flex items-center gap-2"
      @click="$emit('cancelActive')"
    >
      <i class="pi pi-stop-circle text-xs"></i>
      <span class="hidden md:inline">{{ appStore.t('compress.cancel_active') }}</span>
    </button>
    <button
      data-testid="open-global-compression-settings"
      type="button"
      class="h-8 md:h-9 px-3 md:px-5 rounded-lg bg-input border border-subtle text-content text-xs font-bold uppercase tracking-wider hover:bg-primary/10 hover:border-primary transition-all flex items-center gap-2"
      @click="$emit('openSettings')"
    >
      <i class="pi pi-cog text-xs"></i>
      <span class="hidden sm:inline">{{ appStore.t('compress.open_global_settings') }}</span>
    </button>
    <button
      v-if="pendingCount > 0"
      data-testid="start-compression"
      type="button"
      :disabled="busy"
      class="h-8 md:h-9 px-4 md:px-6 rounded-lg bg-primary text-white text-xs font-bold uppercase tracking-wider shadow-lg shadow-primary/25 hover:brightness-110 active:scale-[0.98] transition-all flex items-center gap-2 disabled:opacity-60 disabled:cursor-wait"
      @click="$emit('start')"
    >
      <i
        :class="busy ? 'pi pi-spin pi-spinner' : 'pi pi-play-circle'"
        class="text-xs"
      ></i>
      <span class="hidden sm:inline">{{ appStore.t('compress.start') }}</span>
      <span class="sm:hidden">{{ appStore.t('compress.start_short') }}</span>
    </button>
  </div>
</template>
