<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useAppStore } from '@/stores/app'
import type { CompressionOptions } from '@/stores/compression'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()

interface Props {
  modelValue?: CompressionOptions
  outputPath?: string
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  outputPath: ''
})

interface Emits {
  (e: 'update:modelValue', value: CompressionOptions): void
  (e: 'update:outputPath', value: string): void
}

const emit = defineEmits<Emits>()

const compressionOptions = ref<CompressionOptions>(props.modelValue || {
  format: 'zip',
  level: 6,
  password: '',
  filename: '',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  createSolidArchive: false
})

const outputPath = ref(props.outputPath)
const showAdvanced = ref(false)

const compressionFormats = [
  { value: 'zip', name: 'ZIP' },
  { value: '7z', name: '7Z' },
  { value: 'rar', name: 'RAR' },
  { value: 'tar.gz', name: 'TGZ' },
  { value: 'xz', name: 'XZ' }
]

const selectOutputPath = async () => {
  try {
    const result = await tauriCommands.selectDirectory()
    if (result && typeof result === 'string') {
      outputPath.value = result
      emit('update:outputPath', result)
    }
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

watch(compressionOptions, (newOptions) => {
  emit('update:modelValue', newOptions)
}, { deep: true })

watch(outputPath, (newPath) => {
  emit('update:outputPath', newPath)
})
</script>

<template>
  <div class="horizontal-settings flex flex-col gap-4">
    <!-- 第一行：核心必填参数 -->
    <div class="flex items-center gap-6 flex-wrap lg:flex-nowrap">
      <!-- 格式选择 -->
      <div class="flex flex-col gap-1.5 shrink-0">
        <label class="text-[8px] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.format') }}</label>
        <div class="flex p-1 rounded-xl bg-input border border-subtle gap-1">
          <button 
            v-for="fmt in compressionFormats" :key="fmt.value"
            @click="compressionOptions.format = fmt.value as any"
            class="px-3 py-1.5 rounded-lg text-[9px] font-black transition-all"
            :class="compressionOptions.format === fmt.value ? 'bg-primary text-white shadow-sm' : 'text-dim hover:bg-white/5'"
          >
            {{ fmt.name }}
          </button>
        </div>
      </div>

      <!-- 压缩强度 (精致 Range) -->
      <div class="flex flex-col gap-1.5 flex-1 min-w-[150px]">
        <div class="flex justify-between items-center px-1">
          <label class="text-[8px] font-black text-muted uppercase tracking-widest">{{ appStore.t('compress.level') }}</label>
          <span class="text-[9px] font-mono text-primary font-black">{{ compressionOptions.level }} / 9</span>
        </div>
        <input
          type="range" v-model.number="compressionOptions.level" min="1" max="9" step="1"
          class="w-full h-1 bg-input border border-subtle rounded-full appearance-none cursor-pointer accent-primary"
        />
      </div>

      <!-- 文件名输入 -->
      <div class="flex flex-col gap-1.5 flex-[1.5] min-w-[200px]">
        <label class="text-[8px] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.filename') }}</label>
        <div class="relative">
          <input 
            v-model="compressionOptions.filename" 
            class="w-full px-4 py-2 rounded-xl bg-input border border-subtle text-[11px] text-content outline-none focus:border-primary transition-all placeholder:text-dim"
            :placeholder="appStore.t('vault.placeholder.name')"
          />
          <span class="absolute right-4 top-1/2 -translate-y-1/2 text-[9px] font-mono text-dim uppercase">.{{ compressionOptions.format }}</span>
        </div>
      </div>

      <!-- 高级开关按钮 -->
      <button 
        @click="showAdvanced = !showAdvanced"
        class="mt-auto h-9 px-4 rounded-xl border border-subtle text-[9px] font-black uppercase tracking-widest transition-all"
        :class="showAdvanced ? 'bg-primary/10 border-primary/30 text-primary' : 'bg-input text-muted hover:text-content'"
      >
        <i class="pi pi-cog mr-2" :class="{ 'animate-spin-slow': showAdvanced }"></i>
        Options
      </button>
    </div>

    <!-- 第二行：高级/路径设置 (条件展开) -->
    <transition name="slide-down">
      <div v-if="showAdvanced" class="flex flex-wrap lg:flex-nowrap items-end gap-6 pt-4 border-t border-subtle/30">
        <!-- 目标路径 -->
        <div class="flex flex-col gap-1.5 flex-1 min-w-[300px]">
          <label class="text-[8px] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.output_path') }}</label>
          <div class="flex gap-2">
            <input 
              v-model="outputPath" 
              class="flex-1 px-4 py-2 rounded-xl bg-input border border-subtle text-[10px] text-muted outline-none focus:border-primary transition-all font-mono"
              placeholder="Default System Path"
            />
            <button @click="selectOutputPath" class="w-9 h-9 rounded-xl bg-input border border-subtle flex items-center justify-center hover:bg-primary/10 hover:text-primary transition-all">
              <i class="pi pi-folder text-xs"></i>
            </button>
          </div>
        </div>

        <!-- 密码保护 (极简版) -->
        <div class="flex flex-col gap-1.5 w-48 shrink-0">
          <label class="text-[8px] font-black text-muted uppercase tracking-widest ml-1">Encryption</label>
          <div class="relative">
            <input 
              v-model="compressionOptions.password" type="password"
              class="w-full px-4 py-2 rounded-xl bg-input border border-subtle text-[10px] outline-none focus:border-primary transition-all"
              placeholder="Set Password"
            />
            <i class="pi pi-shield absolute right-3 top-1/2 -translate-y-1/2 text-[9px] text-dim"></i>
          </div>
        </div>

        <!-- 标记开关 -->
        <div class="flex gap-4 mb-1 shrink-0">
          <div v-for="opt in [
            { key: 'keepStructure', icon: 'pi pi-sitemap' },
            { key: 'deleteAfter', icon: 'pi pi-trash' }
          ]" :key="opt.key" 
          @click="(compressionOptions[opt.key as 'keepStructure' | 'deleteAfter'] as boolean) = !compressionOptions[opt.key as 'keepStructure' | 'deleteAfter']"
          class="w-9 h-9 rounded-xl border flex items-center justify-center cursor-pointer transition-all"
          :class="compressionOptions[opt.key as 'keepStructure' | 'deleteAfter'] ? 'bg-primary/20 border-primary text-primary' : 'bg-input border-subtle text-dim hover:text-muted'"
          :title="opt.key">
            <i :class="[opt.icon, 'text-xs']"></i>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.slide-down-enter-active, .slide-down-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.slide-down-enter-from, .slide-down-leave-to { opacity: 0; transform: translateY(-10px); }

.animate-spin-slow {
  animation: spin 3s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
