<script setup lang="ts">
import { computed } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import { useTaskStore } from '@/stores/task'
import { useUpdateStore } from '@/stores/update'
import { useAppStore } from '@/stores/app'

const taskStore = useTaskStore()
const updateStore = useUpdateStore()
const appStore = useAppStore()

const title = computed(() => updateStore.status === 'available' || updateStore.status === 'installing'
  ? appStore.t('update.available').replace('{0}', updateStore.availableVersion)
  : appStore.t('update.title'))
const description = computed(() => updateStore.status === 'available'
  ? appStore.t('update.signed')
  : undefined)
const canInstall = computed(() => updateStore.status === 'available' && taskStore.activeTaskCount === 0)

const close = () => {
  if (updateStore.status !== 'installing') updateStore.remindLater()
}
</script>

<template>
  <Modal
    :visible="updateStore.dialogVisible"
    :title="title"
    :description="description"
    icon="pi pi-cloud-download"
    size="lg"
    :show-close-button="updateStore.status !== 'installing'"
    :show-footer="true"
    :close-on-backdrop="updateStore.status !== 'installing'"
    :close-on-escape="updateStore.status !== 'installing'"
    @update:visible="value => { if (!value) close() }"
  >
    <div class="space-y-5">
      <div v-if="updateStore.status === 'checking'" class="flex items-center gap-3 rounded-2xl border border-primary/20 bg-primary/10 p-5 text-sm text-content">
        <i class="pi pi-spin pi-spinner text-primary"></i>
        {{ appStore.t('update.checking') }}
      </div>

      <template v-else-if="updateStore.manifest">
        <div class="grid grid-cols-2 gap-3 text-xs">
          <div class="rounded-xl border border-subtle bg-input/40 p-4">
            <div class="text-muted">{{ appStore.t('update.latest') }}</div>
            <div class="mt-1 font-black text-content">{{ updateStore.manifest.version }}</div>
          </div>
          <div class="rounded-xl border border-subtle bg-input/40 p-4">
            <div class="text-muted">{{ appStore.t('update.date') }}</div>
            <div class="mt-1 font-black text-content">{{ updateStore.manifest.date || appStore.t('update.date_fallback') }}</div>
          </div>
        </div>
        <div class="rounded-2xl border border-subtle bg-input/30 p-5">
          <div class="mb-3 text-xs font-black uppercase tracking-widest text-muted">{{ appStore.t('update.notes') }}</div>
          <div class="max-h-52 overflow-y-auto whitespace-pre-wrap break-words text-sm leading-6 text-content custom-scrollbar">{{ updateStore.manifest.body || appStore.t('update.notes_fallback') }}</div>
        </div>
      </template>

      <div v-if="taskStore.activeTaskCount > 0" class="flex items-start gap-3 rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs leading-5 text-amber-600">
        <i class="pi pi-exclamation-triangle mt-0.5"></i>
        <span>{{ appStore.t('update.active_tasks').replace('{0}', String(taskStore.activeTaskCount)) }}</span>
      </div>

      <div v-if="updateStore.status === 'installing'" class="flex items-center gap-3 rounded-2xl border border-primary/20 bg-primary/10 p-5 text-sm text-content">
        <i class="pi pi-spin pi-spinner text-primary"></i>
        {{ appStore.t('update.installing') }}
      </div>

      <div v-if="updateStore.status === 'up-to-date'" class="flex items-center gap-3 rounded-2xl border border-emerald-500/25 bg-emerald-500/10 p-5 text-sm text-emerald-600">
        <i class="pi pi-check-circle"></i>
        {{ appStore.t('update.up_to_date') }}
      </div>

      <div v-if="updateStore.status === 'error'" class="rounded-2xl border border-red-500/25 bg-red-500/10 p-5 text-sm leading-6 text-red-500">
        <div class="mb-1 font-black">{{ appStore.t('update.failed') }}</div>
        <div class="break-words">{{ updateStore.errorMessage || appStore.t('update.failed_fallback') }}</div>
      </div>
    </div>

    <template #footer>
      <template v-if="updateStore.status === 'available'">
        <button type="button" class="px-4 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-bold hover:text-content" @click="updateStore.skipCurrentVersion">
          {{ appStore.t('update.skip') }}
        </button>
        <button type="button" class="px-4 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-bold hover:text-content" @click="updateStore.remindLater">
          {{ appStore.t('update.later') }}
        </button>
        <button type="button" :disabled="!canInstall" class="px-5 py-2.5 rounded-xl bg-primary text-white text-xs font-black disabled:cursor-not-allowed disabled:opacity-50" @click="updateStore.install(taskStore.activeTaskCount)">
          {{ appStore.t('update.install') }}
        </button>
      </template>
      <button v-else-if="updateStore.status === 'error'" type="button" class="px-5 py-2.5 rounded-xl bg-primary text-white text-xs font-black" @click="updateStore.checkForUpdates(true)">
        {{ appStore.t('update.retry') }}
      </button>
      <button v-else-if="updateStore.status !== 'checking' && updateStore.status !== 'installing'" type="button" class="px-5 py-2.5 rounded-xl bg-primary text-white text-xs font-black" @click="updateStore.remindLater">
        {{ appStore.t('update.ok') }}
      </button>
    </template>
  </Modal>
</template>
