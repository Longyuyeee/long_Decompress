<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useAppStore } from '@/stores/app'
import type { CompressionOptions } from '@/stores/compression'
import { COMPRESSIBLE_FORMATS, isPasswordSupportedFormat } from '@/utils/compressionFormat'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()

interface Props {
  modelValue?: CompressionOptions
  outputPath?: string
  allowSingleFileFormats?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  outputPath: '',
  allowSingleFileFormats: true
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
let syncingFromProps = false
const showPresetModal = ref(false)
const presetNameInput = ref('')

const compressionFormats = COMPRESSIBLE_FORMATS

// 全部格式支持密码：ZIP/7Z/RAR 原生支持，其他格式通过 7z CLI 自动创建 .7z 加密容器
const supportsPassword = computed(() => isPasswordSupportedFormat(compressionOptions.value.format))

const presets = computed(() => appStore.compressionPresets)

const applyPreset = (preset: { name: string; format: string; level: number; password?: string }) => {
  compressionOptions.value.format = preset.format as any
  compressionOptions.value.level = preset.level
  if (preset.password) compressionOptions.value.password = preset.password
}

const savePreset = () => {
  presetNameInput.value = `${compressionOptions.value.format.toUpperCase()}-L${compressionOptions.value.level}`
  showPresetModal.value = true
}
const confirmSavePreset = () => {
  if (presetNameInput.value.trim()) {
    appStore.saveCompressionPreset(presetNameInput.value.trim(), compressionOptions.value.format, compressionOptions.value.level, compressionOptions.value.password)
  }
  showPresetModal.value = false
}

const isFormatDisabled = (format: { singleFileOnly?: boolean }) => {
  return Boolean(format.singleFileOnly && !props.allowSingleFileFormats)
}

const selectFormat = (format: typeof compressionFormats[number]) => {
  if (isFormatDisabled(format)) return
  compressionOptions.value.format = format.value as CompressionOptions['format']
}

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
  if (syncingFromProps) return
  emit('update:modelValue', newOptions)
}, { deep: true })

watch(() => compressionOptions.value.format, () => {
  if (!supportsPassword.value) {
    compressionOptions.value.password = ''
  }
})

watch(outputPath, (newPath) => {
  if (syncingFromProps) return
  emit('update:outputPath', newPath)
})

watch(() => props.modelValue, (newOptions) => {
  if (!newOptions) return
  syncingFromProps = true
  compressionOptions.value = { ...newOptions }
  Promise.resolve().then(() => {
    syncingFromProps = false
  })
}, { deep: true })

watch(() => props.outputPath, (newPath) => {
  syncingFromProps = true
  outputPath.value = newPath || ''
  Promise.resolve().then(() => {
    syncingFromProps = false
  })
})

watch(() => props.allowSingleFileFormats, (allowSingleFileFormats) => {
  if (allowSingleFileFormats) return
  const currentFormat = compressionFormats.find(format => format.value === compressionOptions.value.format)
  if (currentFormat && isFormatDisabled(currentFormat)) {
    compressionOptions.value.format = 'zip'
  }
})
</script>

<template>
  <div class="horizontal-settings flex flex-col gap-4">
    <!-- 第一行：核心必填参数 -->
    <div class="flex items-center gap-6 flex-wrap lg:flex-nowrap">
      <!-- 格式选择 -->
      <div class="flex flex-col gap-1.5 shrink-0">
        <!-- 预设 -->
        <div v-if="presets.length > 0" class="flex items-center gap-1 mb-1 flex-wrap">
          <span class="text-[0.4375rem] text-dim uppercase font-black tracking-widest shrink-0">{{ appStore.t('preset.label') }}</span>
          <span v-for="(p, i) in presets" :key="i" class="group/preset inline-flex items-center gap-0.5 px-2 py-0.5 rounded text-[0.5rem] font-bold bg-primary/10 border border-primary/20 text-primary">
            <button @click="applyPreset(p)" :title="`${p.format} L${p.level}${p.password ? ' ' + appStore.t('preset.pwd') : ''}`" class="hover:underline">{{ p.name }}</button>
            <button @click="appStore.deleteCompressionPreset(i)" class="ml-0.5 w-3.5 h-3.5 rounded-full flex items-center justify-center text-[0.375rem] text-dim hover:text-red-400 hover:bg-red-500/10 opacity-0 group-hover/preset:opacity-100 transition-opacity" :title="appStore.t('preset.delete')">&times;</button>
          </span>
        </div>

        <label class="text-[0.5rem] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.format') }}</label>
        <div class="flex flex-wrap p-1 rounded-xl bg-input border border-subtle gap-1">
          <button 
            v-for="fmt in compressionFormats" :key="fmt.value"
            @click="selectFormat(fmt)"
            :disabled="isFormatDisabled(fmt)"
            class="px-3 py-1.5 rounded-lg text-[0.5625rem] font-black transition-all"
            :class="[
              compressionOptions.format === fmt.value ? 'bg-primary text-white shadow-sm' : 'text-dim hover:bg-white/5',
              isFormatDisabled(fmt) ? 'opacity-30 cursor-not-allowed hover:bg-transparent' : ''
            ]"
            :title="isFormatDisabled(fmt) ? appStore.t('preset.single_file_only') : fmt.name"
          >
            {{ fmt.name }}
          </button>
        </div>
        <!-- 格式帮助提示 -->
        <div class="text-[0.4375rem] text-dim mt-1 leading-relaxed">
          <span class="text-primary font-bold">ZIP</span> Universal &middot;
          <span class="text-purple-400 font-bold">7Z</span> Best compression &middot;
          <span class="text-amber-400 font-bold">TAR.*</span> Linux archives &middot;
          <span class="font-bold">GZ/BZ2/XZ</span> Single file
        </div>
      </div>

      <!-- 压缩强度 (精致 Range) -->
      <div class="flex flex-col gap-1.5 flex-1 min-w-[150px]">
        <div class="flex justify-between items-center px-1">
          <label class="text-[0.5rem] font-black text-muted uppercase tracking-widest">{{ appStore.t('compress.level') }}</label>
          <span class="text-[0.5625rem] font-mono text-primary font-black">{{ compressionOptions.level }} / 9</span>
        </div>
        <input
          type="range" v-model.number="compressionOptions.level" min="1" max="9" step="1"
          class="w-full h-1 bg-input border border-subtle rounded-full appearance-none cursor-pointer accent-primary"
        />
      </div>

      <!-- 文件名输入 -->
      <div class="flex flex-col gap-1.5 flex-[1.5] min-w-[200px]">
        <label class="text-[0.5rem] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.filename') }}</label>
        <div class="relative">
          <input 
            v-model="compressionOptions.filename" 
            class="w-full px-4 py-2 rounded-xl bg-input border border-subtle text-[0.6875rem] text-content outline-none focus:border-primary transition-all placeholder:text-dim"
            :placeholder="appStore.t('vault.placeholder.name')"
          />
          <span class="absolute right-4 top-1/2 -translate-y-1/2 text-[0.5625rem] font-mono text-dim uppercase">.{{ compressionOptions.format }}</span>
        </div>
      </div>

      <!-- 密码保护 (主行可见) -->
      <div class="flex flex-col gap-1.5 w-32 shrink-0">
        <label class="text-[0.5rem] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('decompress.password') }}</label>
        <div class="relative">
          <input
            v-model="compressionOptions.password" type="password"
            class="w-full px-3 py-2 rounded-xl bg-input border text-[0.625rem] outline-none focus:border-primary transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            :class="compressionOptions.password ? 'border-primary/50' : 'border-subtle'"
            :disabled="!supportsPassword"
            :placeholder="supportsPassword ? (compressionOptions.password ? appStore.t('preset.password_set') : appStore.t('preset.password_optional')) : appStore.t('preset.password_na')"
          />
          <button
            v-if="compressionOptions.password"
            @click="compressionOptions.password = ''"
            class="absolute right-1.5 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full flex items-center justify-center text-dim hover:text-red-400 transition-colors"
          >
            <i class="pi pi-times text-[0.4375rem]"></i>
          </button>
          <i v-else-if="supportsPassword" class="pi pi-lock absolute right-2.5 top-1/2 -translate-y-1/2 text-[0.5625rem] text-dim"></i>
        </div>
      </div>

      <!-- 高级开关按钮 -->
      <button 
        @click="showAdvanced = !showAdvanced"
        class="mt-auto h-9 px-4 rounded-xl border border-subtle text-[0.5625rem] font-black uppercase tracking-widest transition-all"
        :class="showAdvanced ? 'bg-primary/10 border-primary/30 text-primary' : 'bg-input text-muted hover:text-content'"
      >
        <i class="pi pi-cog mr-2" :class="{ 'animate-spin-slow': showAdvanced }"></i>
        {{ appStore.t('preset.options') }}
      </button>
    </div>

    <!-- 第二行：高级/路径设置 (条件展开) -->
    <transition name="slide-down">
      <div v-if="showAdvanced" class="flex flex-wrap lg:flex-nowrap items-end gap-6 pt-4 border-t border-subtle/30">
        <!-- 目标路径 -->
        <div class="flex flex-col gap-1.5 flex-1 min-w-[300px]">
          <label class="text-[0.5rem] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.output_path') }}</label>
          <div class="flex gap-2">
            <input 
              v-model="outputPath" 
              class="flex-1 px-4 py-2 rounded-xl bg-input border border-subtle text-[0.625rem] text-muted outline-none focus:border-primary transition-all font-mono"
              :placeholder="appStore.t('preset.default_path')"
            />
            <button @click="selectOutputPath" class="w-9 h-9 rounded-xl bg-input border border-subtle flex items-center justify-center hover:bg-primary/10 hover:text-primary transition-all">
              <i class="pi pi-folder text-xs"></i>
            </button>
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
          :title="opt.key === 'keepStructure' ? appStore.t('preset.keep_structure') : appStore.t('preset.delete_after')">
            <i :class="[opt.icon, 'text-xs']"></i>
          </div>

          <!-- 分卷开关 -->
          <div
            @click="compressionOptions.splitArchive = !compressionOptions.splitArchive"
            class="w-9 h-9 rounded-xl border flex items-center justify-center cursor-pointer transition-all"
            :class="compressionOptions.splitArchive ? 'bg-primary/20 border-primary text-primary' : 'bg-input border-subtle text-dim hover:text-muted'"
            :title="appStore.t('preset.split_archive')">
            <i class="pi pi-clone text-xs"></i>
          </div>
          <input
            v-if="compressionOptions.splitArchive"
            v-model.number="compressionOptions.splitSize"
            type="number" min="1" step="1"
            class="w-16 px-2 py-2 rounded-xl bg-input border border-subtle text-[0.625rem] text-content outline-none focus:border-primary transition-all font-mono"
            :placeholder="appStore.t('preset.mb')"
          />
        </div>
      </div>
    </transition>
  </div>
<!-- 预设名称弹窗 -->
<transition name="pop">
  <div v-if="showPresetModal" class="fixed inset-0 z-[200] flex items-center justify-center bg-black/60 backdrop-blur-md p-4" @click.self="showPresetModal = false">
    <div class="modal-no-glass rounded-[2rem] p-8 w-full max-w-sm shadow-2xl text-content">
      <h3 class="text-sm font-black mb-4 uppercase tracking-widest">{{ appStore.t('preset.name_prompt') }}</h3>
      <input v-model="presetNameInput" @keyup.enter="confirmSavePreset" class="w-full h-10 rounded-xl bg-input border border-subtle px-4 text-[0.625rem] font-mono text-content outline-none focus:border-primary transition-all mb-4" autofocus />
      <div class="flex gap-2">
        <button @click="showPresetModal = false" class="flex-1 py-2.5 rounded-xl bg-input border border-subtle text-muted text-[0.5625rem] font-black uppercase tracking-widest hover:text-content transition-all">{{ appStore.t('vault.confirm.cancel') }}</button>
        <button @click="confirmSavePreset" class="flex-1 py-2.5 rounded-xl bg-primary text-white text-[0.5625rem] font-black uppercase tracking-widest hover:brightness-110 transition-all">{{ appStore.t('preset.name_prompt') }}</button>
      </div>
    </div>
  </div>
</transition>
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

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.95) translateY(10px); }
</style>
