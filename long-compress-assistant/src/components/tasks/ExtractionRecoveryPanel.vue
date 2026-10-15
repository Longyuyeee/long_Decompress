<script setup lang="ts">
import Modal from '@/components/ui/Modal.vue'
defineProps<{ inline?: boolean; busy: boolean }>()
defineEmits<{ close: [] }>()
</script>

<template>
  <section v-if="inline" class="min-w-0 rounded-xl border border-subtle bg-input/20 p-3" data-testid="inline-extraction-recovery">
    <div class="mb-3 flex items-center gap-2 text-sm font-bold text-content"><i class="pi pi-wrench text-primary"></i>处理这条任务</div>
    <slot />
    <div class="mt-3 border-t border-subtle pt-3"><slot name="footer" /></div>
  </section>
  <Modal v-else :visible="true" title="处理本批失败项" size="lg" :show-close-button="!busy"
    :close-on-backdrop="false" :close-on-escape="!busy" @close="$emit('close')">
    <slot />
    <template #footer><slot name="footer" /></template>
  </Modal>
</template>
