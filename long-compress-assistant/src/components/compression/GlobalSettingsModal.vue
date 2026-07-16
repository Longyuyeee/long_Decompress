<script setup lang="ts">
import { ref } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import CompressionSettingsPanel from './CompressionSettingsPanel.vue'
import type { CompressionOptions } from '@/stores/compression'

interface Props {
  visible: boolean
  settings: CompressionOptions
  outputPath: string
  allowSingleFileFormats: boolean
}

interface Emits {
  (e: 'update:visible', value: boolean): void
  (e: 'update:settings', value: CompressionOptions): void
  (e: 'update:outputPath', value: string): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const localSettings = ref<CompressionOptions>({ ...props.settings })
const localOutputPath = ref(props.outputPath)

const handleClose = () => {
  emit('update:visible', false)
}

const handleSave = () => {
  emit('update:settings', localSettings.value)
  emit('update:outputPath', localOutputPath.value)
  emit('update:visible', false)
}

const handleCancel = () => {
  // 恢复原始值
  localSettings.value = { ...props.settings }
  localOutputPath.value = props.outputPath
  emit('update:visible', false)
}
</script>

<template>
  <Modal
    :visible="visible"
    @close="handleCancel"
    :width="700"
  >
    <template #header>
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-primary/20 flex items-center justify-center">
          <i class="pi pi-cog text-primary text-lg"></i>
        </div>
        <div>
          <h3 class="text-lg font-black text-content uppercase tracking-wider">全局压缩设置</h3>
          <p class="text-xs text-muted uppercase tracking-widest mt-1">应用到所有未单独配置的文件和组</p>
        </div>
      </div>
    </template>

    <div class="p-6 space-y-6">
      <!-- 复用设置面板组件 -->
      <CompressionSettingsPanel
        v-model="localSettings"
        v-model:outputPath="localOutputPath"
        :allow-single-file-formats="allowSingleFileFormats"
      />
    </div>

    <template #footer>
      <div class="flex gap-3 justify-end px-6 pb-6">
        <button
          @click="handleCancel"
          class="px-6 py-2.5 rounded-xl bg-input border border-subtle text-content text-sm font-bold uppercase tracking-wider hover:border-primary transition-all"
        >
          取消
        </button>
        <button
          @click="handleSave"
          class="px-6 py-2.5 rounded-xl bg-primary text-white text-sm font-bold uppercase tracking-wider hover:bg-primary/90 transition-all shadow-lg shadow-primary/25"
        >
          保存设置
        </button>
      </div>
    </template>
  </Modal>
</template>
