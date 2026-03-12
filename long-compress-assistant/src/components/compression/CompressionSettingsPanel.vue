<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useAppStore } from '@/stores/app'

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
  (e: 'format-changed', format: string): void
  (e: 'options-changed', options: CompressionOptions): void
}

const emit = defineEmits<Emits>()

export interface CompressionOptions {
  format: 'zip' | '7z' | 'tar' | 'gz' | 'bz2' | 'tar.gz' | 'tar.bz2' | 'xz' | 'tar.xz' | 'rar'
  level: number
  password: string
  filename: string
  splitArchive: boolean
  splitSize: string
  keepStructure: boolean
  deleteAfter: boolean
  createSolidArchive: boolean
}

const showPassword = ref(false)
const showConfirmPassword = ref(false)
const confirmPassword = ref('')

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

const compressionFormats = [
  { value: 'zip', name: 'ZIP', icon: 'pi pi-file', color: 'text-primary', description: 'Universal', extension: 'zip' },
  { value: '7z', name: '7-Zip', icon: 'pi pi-file', color: 'text-purple-500', description: 'High Ratio', extension: '7z' },
  { value: 'rar', name: 'RAR', icon: 'pi pi-file', color: 'text-red-500', description: 'Proprietary', extension: 'rar' },
  { value: 'tar.gz', name: 'TAR.GZ', icon: 'pi pi-file', color: 'text-blue-500', description: 'Linux Std', extension: 'tar.gz' },
  { value: 'xz', name: 'XZ', icon: 'pi pi-file', color: 'text-indigo-500', description: 'Extreme', extension: 'xz' },
  { value: 'tar', name: 'TAR', icon: 'pi pi-file', color: 'text-slate-500', description: 'Archive', extension: 'tar' },
  { value: 'gz', name: 'GZIP', icon: 'pi pi-file', color: 'text-orange-500', description: 'Web Std', extension: 'gz' },
  { value: 'bz2', name: 'BZIP2', icon: 'pi pi-file', color: 'text-emerald-500', description: 'Solid', extension: 'bz2' },
  { value: 'tar.bz2', name: 'TAR.BZ2', icon: 'pi pi-file', color: 'text-teal-500', description: 'Hybrid', extension: 'tar.bz2' },
  { value: 'tar.xz', name: 'TAR.XZ', icon: 'pi pi-file', color: 'text-cyan-500', description: 'Advanced', extension: 'tar.xz' }
]

const supportsSplitArchive = computed(() => ['zip', '7z', 'rar'].includes(compressionOptions.value.format))
const supportsSolidArchive = computed(() => compressionOptions.value.format === '7z')

const selectFormat = (format: string) => {
  compressionOptions.value.format = format as any
  emit('format-changed', format)
}

const selectOutputPath = async () => {
  try {
    const result = await tauriCommands.selectDirectory()
    if (result && typeof result === 'string') {
      outputPath.value = result
      emit('update:outputPath', result)
    }
  } catch (error) {
    appStore.setError(appStore.language === 'zh-CN' ? '选择路径失败' : 'Failed to select path')
  }
}

const getCurrentFormatExtension = (): string => {
  const selectedFormat = compressionFormats.find(f => f.value === compressionOptions.value.format)
  return selectedFormat?.extension || compressionOptions.value.format
}

watch(compressionOptions, (newOptions) => {
  emit('update:modelValue', newOptions)
  emit('options-changed', newOptions)
}, { deep: true })

watch(outputPath, (newPath) => {
  emit('update:outputPath', newPath)
})

watch(() => compressionOptions.value.format, (newFormat) => {
  if (['tar', 'gz', 'bz2'].includes(newFormat)) {
    compressionOptions.value.password = ''
    compressionOptions.value.splitArchive = false
  }
})

defineExpose({
  getOptions: () => compressionOptions.value,
  getOutputPath: () => outputPath.value,
  validate: () => {
    if (compressionOptions.value.password && compressionOptions.value.password !== confirmPassword.value) {
      return { valid: false, error: 'Passwords do not match' }
    }
    return { valid: true }
  }
})
</script>

<template>
  <div class="compression-settings-panel space-y-8 pb-10">
    <!-- 格式选择网格 -->
    <section class="aero-card p-8 transition-all duration-500">
      <h2 class="text-[10px] font-black text-muted uppercase tracking-[0.3em] mb-8">{{ appStore.t('compress.format') }}</h2>
      
      <div class="grid grid-cols-2 sm:grid-cols-5 gap-4">
        <button
          v-for="format in compressionFormats"
          :key="format.value"
          @click="selectFormat(format.value)"
          class="group p-4 rounded-2xl border transition-all duration-500 text-center relative overflow-hidden"
          :class="compressionOptions.format === format.value
            ? 'border-primary bg-primary/10 shadow-[0_0_20px_color-mix(in_srgb,var(--dynamic-accent),transparent_80%)]'
            : 'border-subtle bg-input hover:border-primary/50'"
        >
          <div class="absolute inset-0 bg-primary opacity-0 group-hover:opacity-5 transition-opacity"></div>
          <i :class="[format.icon, format.color]" class="text-2xl block mb-3 group-hover:scale-110 transition-transform"></i>
          <span class="font-black text-xs text-content tracking-widest">{{ format.name }}</span>
          <p class="text-[8px] text-muted mt-1 uppercase tracking-tighter">{{ format.description }}</p>
        </button>
      </div>
    </section>

    <!-- 核心参数配置 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
      <!-- 压缩级别 -->
      <section class="aero-card p-8">
        <div class="flex justify-between items-center mb-10">
           <h2 class="text-[10px] font-black text-muted uppercase tracking-[0.3em]">{{ appStore.t('compress.level') }}</h2>
           <span class="px-3 py-1 rounded-full bg-primary/10 border border-primary/20 text-primary text-[10px] font-black font-mono shadow-sm">
             {{ compressionOptions.level }} / 9
           </span>
        </div>
        
        <div class="space-y-8">
          <div class="relative pt-2">
            <input
              type="range"
              v-model="compressionOptions.level"
              min="1" max="9" step="1"
              class="w-full h-1.5 bg-input border border-subtle rounded-full appearance-none cursor-pointer"
            />
            <div class="flex justify-between text-[8px] text-muted font-black uppercase tracking-widest mt-4">
              <span>Fastest</span>
              <span>Balanced</span>
              <span>Extreme</span>
            </div>
          </div>
          
          <div class="p-4 rounded-2xl bg-input border border-subtle">
             <div class="text-[9px] text-muted uppercase tracking-widest mb-1">{{ appStore.t('compress.estimated_size') }}</div>
             <div class="text-2xl font-black text-content tracking-tighter">
               {{ (100 - (compressionOptions.level * 8)).toFixed(1) }}% <span class="text-xs text-dim ml-2 font-normal italic">of source</span>
             </div>
          </div>
        </div>
      </section>

      <!-- 密码保护 -->
      <section class="aero-card p-8">
        <h2 class="text-[10px] font-black text-muted uppercase tracking-[0.3em] mb-8">Security & Encryption</h2>
        <div class="space-y-4">
          <div class="relative group">
            <input
              :type="showPassword ? 'text' : 'password'"
              v-model="compressionOptions.password"
              class="w-full px-6 py-4 rounded-2xl bg-input border border-subtle text-content text-sm focus:border-primary transition-all outline-none placeholder:text-dim"
              placeholder="Set Encryption Password"
            />
            <button @click="showPassword = !showPassword" class="absolute right-6 top-1/2 -translate-y-1/2 text-dim hover:text-content">
              <i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye'"></i>
            </button>
          </div>
          
          <transition name="fade">
            <div v-if="compressionOptions.password" class="relative">
              <input
                :type="showConfirmPassword ? 'text' : 'password'"
                v-model="confirmPassword"
                class="w-full px-6 py-4 rounded-2xl bg-input border border-subtle text-content text-sm focus:border-primary transition-all outline-none placeholder:text-dim"
                placeholder="Confirm Password"
              />
            </div>
          </transition>
        </div>
      </section>
    </div>

    <!-- 路径与高级设置 -->
    <section class="aero-card p-8">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-12">
        <div class="space-y-6">
          <h2 class="text-[10px] font-black text-muted uppercase tracking-[0.3em]">Deployment Path</h2>
          <div class="flex gap-3">
             <input v-model="outputPath" class="flex-1 px-6 py-4 rounded-2xl bg-input border border-subtle text-content text-xs outline-none focus:border-primary transition-all" placeholder="Target Directory" />
             <button @click="selectOutputPath" class="w-14 h-14 rounded-2xl bg-input border border-subtle flex items-center justify-center hover:bg-primary/20 hover:text-primary transition-all text-muted">
               <i class="pi pi-folder"></i>
             </button>
          </div>
          <div class="flex gap-3 items-center">
             <input v-model="compressionOptions.filename" class="flex-1 px-6 py-4 rounded-2xl bg-input border border-subtle text-content text-xs outline-none focus:border-primary transition-all" placeholder="Archive Filename" />
             <span class="text-dim font-mono text-[10px] uppercase">.{{ getCurrentFormatExtension() }}</span>
          </div>
        </div>

        <div class="space-y-6">
          <h2 class="text-[10px] font-black text-muted uppercase tracking-[0.3em]">Advanced Flags</h2>
          <div class="grid grid-cols-1 gap-3">
             <label v-for="opt in [
               { key: 'keepStructure', label: 'Maintain Directory Tree' },
               { key: 'deleteAfter', label: 'Auto-Purge Source' },
               { key: 'createSolidArchive', label: 'Solid Compression', disabled: !supportsSolidArchive }
             ]" :key="opt.key" class="flex items-center justify-between p-4 rounded-2xl bg-input border border-subtle group cursor-pointer transition-all hover:bg-primary/5" :class="{ 'opacity-30 pointer-events-none': opt.disabled }">
               <span class="text-[11px] text-content font-bold tracking-tight">{{ opt.label }}</span>
               <div class="w-10 h-5 rounded-full border border-subtle p-0.5 transition-all" :class="compressionOptions[opt.key as keyof CompressionOptions] ? 'bg-primary border-primary' : 'bg-card'">
                 <input type="checkbox" v-model="compressionOptions[opt.key as keyof CompressionOptions]" class="hidden" />
                 <div class="w-3.5 h-3.5 rounded-full bg-white transition-all shadow-sm" :class="compressionOptions[opt.key as keyof CompressionOptions] ? 'translate-x-5' : ''"></div>
               </div>
             </label>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.5s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: white;
  border: 4px solid var(--dynamic-accent);
  box-shadow: 0 0 10px color-mix(in srgb, var(--dynamic-accent), transparent 50%);
  cursor: pointer;
}
</style>
