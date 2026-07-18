<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useAppStore } from '@/stores/app'
import { useCompressionProfileStore } from '@/stores/compressionProfile'
import type { CompressionOptions } from '@/stores/compression'
import type { CompressionProfile } from '@/types/profile'
import { COMPRESSIBLE_FORMATS, isPasswordSupportedFormat } from '@/utils/compressionFormat'
import ProfileSelector from '@/components/profiles/ProfileSelector.vue'
import ProfileManager from '@/components/profiles/ProfileManager.vue'
import PasswordGeneratorDialog from '@/components/password/PasswordGeneratorDialog.vue'
import { extractErrorMessage } from '@/utils'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()
const profileStore = useCompressionProfileStore()

const showPasswordGenerator = ref(false)

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
const showProfileSelector = ref(false)
const profileDialogMode = ref<'select' | 'manage'>('select')
let syncingFromProps = false
const showPresetModal = ref(false)
const presetNameInput = ref('')
const showSaveProfileModal = ref(false)
const newProfileName = ref('')
const newProfileIcon = ref('📦')
const newProfileDescription = ref('')

const compressionFormats = COMPRESSIBLE_FORMATS
const selectedFormat = computed(() => compressionFormats.find(format => format.value === compressionOptions.value.format))

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

const applyProfile = (profile: CompressionProfile) => {
  compressionOptions.value.format = profile.config.format as any
  compressionOptions.value.level = profile.config.level
  compressionOptions.value.password = profile.config.password || ''
  compressionOptions.value.splitArchive = profile.config.splitArchive
  compressionOptions.value.splitSize = profile.config.splitSize?.toString() || '1024'
  compressionOptions.value.keepStructure = profile.config.keepStructure
  compressionOptions.value.deleteAfter = profile.config.deleteAfter
  compressionOptions.value.createSolidArchive = profile.config.createSolidArchive
  showProfileSelector.value = false
  appStore.setSuccess(appStore.t('profiles.applied_success').replace('{0}', profile.name))
}

const openSaveProfileModal = () => {
  newProfileName.value = `${compressionOptions.value.format.toUpperCase()}-L${compressionOptions.value.level}`
  newProfileIcon.value = '📦'
  newProfileDescription.value = ''
  showSaveProfileModal.value = true
}

const saveAsNewProfile = async () => {
  if (!newProfileName.value.trim()) {
    appStore.setError(appStore.t('profiles.name_required'))
    return
  }

  try {
    await profileStore.addProfile({
      name: newProfileName.value.trim(),
      icon: newProfileIcon.value,
      description: newProfileDescription.value.trim(),
      config: {
        format: compressionOptions.value.format,
        level: compressionOptions.value.level,
        password: compressionOptions.value.password || null,
        splitArchive: compressionOptions.value.splitArchive,
        splitSize: compressionOptions.value.splitArchive ? parseInt(compressionOptions.value.splitSize) : null,
        keepStructure: compressionOptions.value.keepStructure,
        deleteAfter: compressionOptions.value.deleteAfter,
        createSolidArchive: compressionOptions.value.createSolidArchive,
        filenameTemplate: compressionOptions.value.filename ? `{name}_${compressionOptions.value.filename}` : null,
        extraParams: {}
      }
    })
    showSaveProfileModal.value = false
    appStore.setSuccess(appStore.t('profiles.save_success'))
  } catch (error) {
    appStore.setError(`${appStore.t('profiles.save_failed')}: ${extractErrorMessage(error)}`)
  }
}

const iconOptions = ['📦', '🗜️', '📁', '🔐', '⚡', '🎯', '💼', '🎨', '🔧', '⭐']

const handlePasswordGenerated = (password: string) => {
  compressionOptions.value.password = password
  showPasswordGenerator.value = false
}

</script>

<template>
  <div class="horizontal-settings flex flex-col gap-4">
    <!-- 第一行：核心必填参数 -->
    <div class="settings-core-grid">
      <!-- 格式选择 -->
      <div class="flex flex-col gap-1.5 min-w-0">
        <!-- 预设 -->
        <div v-if="presets.length > 0" class="flex items-center gap-1 mb-1 flex-wrap">
          <span class="text-sm text-dim uppercase font-black tracking-widest shrink-0">{{ appStore.t('preset.label') }}</span>
          <span v-for="(p, i) in presets" :key="i" class="group/preset inline-flex items-center gap-0.5 px-2 py-0.5 rounded text-xs font-bold bg-primary/10 border border-primary/20 text-primary">
            <button @click="applyPreset(p)" :title="`${p.format} L${p.level}${p.password ? ' ' + appStore.t('preset.pwd') : ''}`" class="hover:underline">{{ p.name }}</button>
            <button @click="appStore.deleteCompressionPreset(i)" class="ml-0.5 w-3.5 h-3.5 rounded-full flex items-center justify-center text-[0.375rem] text-dim hover:text-red-400 hover:bg-red-500/10 opacity-0 group-hover/preset:opacity-100 transition-opacity" :title="appStore.t('preset.delete')">&times;</button>
          </span>
        </div>

        <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.format') }}</label>
        <div class="relative">
          <select
            v-model="compressionOptions.format"
            class="w-full h-10 appearance-none rounded-xl bg-input border border-subtle pl-4 pr-10 text-sm font-black text-content outline-none focus:border-primary"
          >
            <option
              v-for="fmt in compressionFormats"
              :key="fmt.value"
              :value="fmt.value"
              :disabled="isFormatDisabled(fmt)"
            >{{ fmt.name }}{{ fmt.singleFileOnly ? ` · ${appStore.t('preset.single_file_only')}` : '' }}</option>
          </select>
          <i class="pi pi-chevron-down absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted pointer-events-none"></i>
        </div>
        <p class="text-xs text-dim leading-relaxed truncate" :title="selectedFormat?.name">
          {{ compressionOptions.format === 'zip' ? '兼容性最好，适合分享' : compressionOptions.format === '7z' ? '压缩率更高，适合归档' : compressionOptions.format.startsWith('tar') ? '适合 Linux / Unix 环境' : '仅适合单个文件' }}
        </p>
      </div>

      <!-- 压缩强度 (精致 Range) -->
      <div class="flex flex-col gap-1.5 min-w-0">
        <div class="flex justify-between items-center px-1">
          <label class="text-xs font-black text-muted uppercase tracking-widest">{{ appStore.t('compress.level') }}</label>
          <span class="text-xs font-mono text-primary font-black">{{ compressionOptions.level }} / 9</span>
        </div>
        <input
          type="range" v-model.number="compressionOptions.level" min="1" max="9" step="1"
          class="w-full h-1 bg-input border border-subtle rounded-full appearance-none cursor-pointer accent-primary"
        />
      </div>

      <!-- 文件名输入 -->
      <div class="flex flex-col gap-1.5 min-w-0">
        <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.filename') }}</label>
        <div class="relative">
          <input 
            v-model="compressionOptions.filename" 
            class="w-full px-4 py-2 rounded-xl bg-input border border-subtle text-sm text-content outline-none focus:border-primary transition-all placeholder:text-dim"
            :placeholder="appStore.t('vault.placeholder.name')"
          />
          <span class="absolute right-4 top-1/2 -translate-y-1/2 text-xs font-mono text-dim uppercase">.{{ compressionOptions.format }}</span>
        </div>
      </div>

      <!-- 密码保护 (主行可见) 带生成器按钮 -->
      <div class="flex flex-col gap-1.5 min-w-0">
        <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('decompress.password') }}</label>
        <div class="flex gap-1">
          <div class="relative flex-1">
            <input
              v-model="compressionOptions.password" type="password"
              class="w-full px-3 py-2 rounded-xl bg-input border text-sm outline-none focus:border-primary transition-all disabled:opacity-85 disabled:cursor-not-allowed"
              :class="compressionOptions.password ? 'border-primary/50' : 'border-subtle'"
              :disabled="!supportsPassword"
              :placeholder="supportsPassword ? (compressionOptions.password ? appStore.t('preset.password_set') : appStore.t('preset.password_optional')) : appStore.t('preset.password_na')"
            />
            <button
              v-if="compressionOptions.password"
              @click="compressionOptions.password = ''"
              class="absolute right-1.5 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full flex items-center justify-center text-dim hover:text-red-400 transition-colors"
            >
              <i class="pi pi-times text-sm"></i>
            </button>
            <i v-else-if="supportsPassword" class="pi pi-lock absolute right-2.5 top-1/2 -translate-y-1/2 text-xs text-dim"></i>
          </div>
          <button
            v-if="supportsPassword"
            @click.stop="showPasswordGenerator = true"
            type="button"
            class="w-9 h-9 rounded-xl bg-primary/10 hover:bg-primary/20 border border-primary/20 text-primary transition-colors flex items-center justify-center shrink-0"
            :title="appStore.t('password.generator.title', '密码生成器')"
          >
            <span class="text-base">🎲</span>
          </button>
        </div>
      </div>

    </div>

    <div class="flex flex-wrap items-center gap-2">
      <!-- 高级开关按钮 -->
      <button
        @click="showAdvanced = !showAdvanced"
        class="h-9 px-4 rounded-xl border border-subtle text-xs font-black transition-all whitespace-nowrap"
        :class="showAdvanced ? 'bg-primary/10 border-primary/30 text-primary' : 'bg-input text-muted hover:text-content'"
      >
        <i class="pi pi-cog mr-2" :class="{ 'animate-spin-slow': showAdvanced }"></i>
        {{ appStore.t('preset.advanced_options', '输出与高级选项') }}
      </button>

      <!-- 配置组选择按钮 -->
      <button
        @click="profileDialogMode = 'manage'; showProfileSelector = true"
        class="h-9 px-4 rounded-xl border border-subtle text-xs font-black transition-all whitespace-nowrap"
        :class="showProfileSelector ? 'bg-sky-500/10 border-sky-500/30 text-sky-400' : 'bg-input text-muted hover:text-content'"
      >
        <i class="pi pi-bookmark mr-2"></i>
        {{ appStore.t('profiles.manage') }}
      </button>

      <!-- 保存为配置组按钮 -->
      <button
        @click="openSaveProfileModal"
        class="h-9 px-4 rounded-xl border border-subtle text-xs font-black transition-all bg-input text-muted hover:text-content hover:border-sky-500/30 whitespace-nowrap"
        :title="appStore.t('profiles.save_as_new')"
      >
        <i class="pi pi-save mr-2"></i>
        {{ appStore.t('profiles.save') }}
      </button>
    </div>

    <!-- 第二行：高级/路径设置 (条件展开) -->
    <transition name="slide-down">
      <div v-if="showAdvanced" class="space-y-4 pt-4 border-t border-subtle/30">
        <!-- 目标路径 -->
        <div class="flex flex-col gap-1.5 min-w-0">
          <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.output_path') }}</label>
          <div class="flex gap-2">
            <input 
              v-model="outputPath" 
              class="flex-1 px-4 py-2 rounded-xl bg-input border border-subtle text-sm text-muted outline-none focus:border-primary transition-all font-mono"
              :placeholder="appStore.t('preset.default_path')"
            />
            <button @click="selectOutputPath" class="w-9 h-9 rounded-xl bg-input border border-subtle flex items-center justify-center hover:bg-primary/10 hover:text-primary transition-all">
              <i class="pi pi-folder text-xs"></i>
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
          <label class="advanced-option">
            <input v-model="compressionOptions.keepStructure" type="checkbox" />
            <span><strong>{{ appStore.t('preset.keep_structure') }}</strong><small>保留源文件夹的层级关系</small></span>
          </label>
          <label class="advanced-option advanced-option-danger">
            <input v-model="compressionOptions.deleteAfter" type="checkbox" />
            <span><strong>{{ appStore.t('preset.delete_after') }}</strong><small>仅压缩成功后删除原始文件，请谨慎启用</small></span>
          </label>
          <label class="advanced-option">
            <input v-model="compressionOptions.splitArchive" type="checkbox" />
            <span><strong>{{ appStore.t('preset.split_archive') }}</strong><small>把大压缩包拆成多个便于传输的分卷</small></span>
          </label>
          <label v-if="compressionOptions.format === '7z'" class="advanced-option">
            <input v-model="compressionOptions.createSolidArchive" type="checkbox" />
            <span><strong>固实压缩</strong><small>提高同类文件压缩率，但单文件提取更慢</small></span>
          </label>
          <label v-if="compressionOptions.splitArchive" class="advanced-option">
            <span class="w-full"><strong>每个分卷大小</strong><small>单位：MB</small></span>
            <input v-model.number="compressionOptions.splitSize" type="number" min="1" step="1" class="w-24 h-9 px-3 rounded-lg bg-input border border-subtle text-sm text-content outline-none focus:border-primary" />
          </label>
        </div>
      </div>
    </transition>
  </div>

  <!-- 配置组管理弹窗 -->
  <Teleport to="body">
    <transition name="pop">
      <div v-if="showProfileSelector" class="fixed inset-0 z-[320] flex items-center justify-center bg-black/55 p-4" @click.self="showProfileSelector = false">
        <div class="modal-no-glass w-full max-w-3xl max-h-[82vh] overflow-y-auto rounded-2xl p-6 text-content shadow-2xl">
          <div class="flex items-center justify-between mb-5">
            <div>
              <h3 class="text-base font-black">{{ profileDialogMode === 'select' ? '选择压缩配置' : '管理压缩配置组' }}</h3>
              <p class="text-xs text-muted mt-1">{{ profileDialogMode === 'select' ? '选择后会立即应用格式、压缩级别和高级选项' : '创建、修改或删除可重复使用的配置组' }}</p>
            </div>
            <button type="button" class="w-8 h-8 rounded-lg bg-input text-muted hover:text-content" @click="showProfileSelector = false"><i class="pi pi-times"></i></button>
          </div>
          <ProfileSelector v-if="profileDialogMode === 'select'" :show-manage-button="true" @apply="applyProfile" @manage="profileDialogMode = 'manage'" />
          <div v-else>
            <button type="button" class="mb-4 h-9 px-4 rounded-lg bg-input border border-subtle text-xs font-bold text-muted hover:text-content" @click="profileDialogMode = 'select'"><i class="pi pi-arrow-left mr-2"></i>返回选择配置</button>
            <ProfileManager />
          </div>
        </div>
      </div>
    </transition>
  </Teleport>
<Teleport to="body">
<!-- 预设名称弹窗 -->
<transition name="pop">
  <div v-if="showPresetModal" class="fixed inset-0 z-[330] flex items-center justify-center bg-black/65 p-4" @click.self="showPresetModal = false">
    <div class="modal-no-glass rounded-[2rem] p-8 w-full max-w-sm shadow-2xl text-content">
      <h3 class="text-sm font-black mb-4 uppercase tracking-widest">{{ appStore.t('preset.name_prompt') }}</h3>
      <input v-model="presetNameInput" @keyup.enter="confirmSavePreset" class="w-full h-10 rounded-xl bg-input border border-subtle px-4 text-sm font-mono text-content outline-none focus:border-primary transition-all mb-4" autofocus />
      <div class="flex gap-2">
        <button @click="showPresetModal = false" class="flex-1 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-black uppercase tracking-widest hover:text-content transition-all">{{ appStore.t('vault.confirm.cancel') }}</button>
        <button @click="confirmSavePreset" class="flex-1 py-2.5 rounded-xl bg-primary text-white text-xs font-black uppercase tracking-widest hover:brightness-110 transition-all">{{ appStore.t('preset.name_prompt') }}</button>
      </div>
    </div>
  </div>
</transition>

<!-- 保存为配置组弹窗 -->
<transition name="pop">
  <div v-if="showSaveProfileModal" class="fixed inset-0 z-[330] flex items-center justify-center bg-black/65 p-4" @click.self="showSaveProfileModal = false">
    <div class="modal-no-glass rounded-[2rem] p-8 w-full max-w-md shadow-2xl text-content">
      <h3 class="text-sm font-black mb-4 uppercase tracking-widest">{{ appStore.t('profiles.save_as_profile') }}</h3>

      <!-- 图标选择 -->
      <div class="mb-4">
        <label class="block text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('profiles.icon_label') }}</label>
        <div class="flex gap-2 flex-wrap">
          <button
            v-for="icon in iconOptions"
            :key="icon"
            @click="newProfileIcon = icon"
            class="w-10 h-10 text-2xl rounded-lg transition-all"
            :class="newProfileIcon === icon ? 'bg-sky-500/20 ring-2 ring-sky-500' : 'bg-input border border-subtle hover:bg-white/5'"
          >
            {{ icon }}
          </button>
        </div>
      </div>

      <!-- 名称 -->
      <div class="mb-4">
        <label class="block text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('profiles.name_label') }}</label>
        <input
          v-model="newProfileName"
          @keyup.enter="saveAsNewProfile"
          class="w-full h-10 rounded-xl bg-input border border-subtle px-4 text-sm font-mono text-content outline-none focus:border-primary transition-all"
          autofocus
        />
      </div>

      <!-- 描述 -->
      <div class="mb-4">
        <label class="block text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('profiles.desc_optional') }}</label>
        <textarea
          v-model="newProfileDescription"
          rows="2"
          class="w-full rounded-xl bg-input border border-subtle px-4 py-2 text-sm text-content outline-none focus:border-primary transition-all resize-none"
          :placeholder="appStore.t('profiles.desc_placeholder')"
        ></textarea>
      </div>

      <!-- 当前配置预览 -->
      <div class="mb-4 p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
        <p class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('profiles.current_config') }}</p>
        <div class="flex flex-wrap gap-2 text-xs">
          <span class="px-2 py-1 bg-slate-700/50 rounded text-slate-300">{{ compressionOptions.format.toUpperCase() }}</span>
          <span class="px-2 py-1 bg-slate-700/50 rounded text-slate-300">L{{ compressionOptions.level }}</span>
          <span v-if="compressionOptions.password" class="px-2 py-1 bg-sky-500/10 text-sky-400 rounded">{{ appStore.t('profiles.badge_encrypted') }}</span>
          <span v-if="compressionOptions.splitArchive" class="px-2 py-1 bg-purple-500/10 text-purple-400 rounded">{{ appStore.t('profiles.badge_split') }}</span>
          <span v-if="compressionOptions.createSolidArchive" class="px-2 py-1 bg-amber-500/10 text-amber-400 rounded">{{ appStore.t('profiles.badge_solid') }}</span>
        </div>
      </div>

      <div class="flex gap-2">
        <button @click="showSaveProfileModal = false" class="flex-1 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-black uppercase tracking-widest hover:text-content transition-all">{{ appStore.t('vault.confirm.cancel') }}</button>
        <button @click="saveAsNewProfile" class="flex-1 py-2.5 rounded-xl bg-sky-500 text-white text-xs font-black uppercase tracking-widest hover:brightness-110 transition-all">{{ appStore.t('profiles.save_button') }}</button>
      </div>
    </div>
  </div>
</transition>
</Teleport>

<!-- 密码生成器对话框 -->
<PasswordGeneratorDialog
  :is-open="showPasswordGenerator"
  @close="showPasswordGenerator = false"
  @select="handlePasswordGenerated"
/>
</template>

<style scoped>
.settings-core-grid {
  display: grid;
  grid-template-columns: minmax(11rem, 1.05fr) minmax(9rem, 0.75fr) minmax(11rem, 1fr) minmax(11rem, 1fr);
  gap: 1rem;
  align-items: end;
}

.advanced-option {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  border: 1px solid var(--border-subtle);
  border-radius: 0.75rem;
  background: color-mix(in srgb, var(--bg-input) 55%, transparent);
  cursor: pointer;
}

.advanced-option input[type='checkbox'] {
  width: 1rem;
  height: 1rem;
  flex: none;
  accent-color: var(--dynamic-accent);
}

.advanced-option span {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.15rem;
}

.advanced-option strong {
  color: var(--text-base);
  font-size: 0.75rem;
}

.advanced-option small {
  color: var(--text-muted);
  font-size: 0.6875rem;
  line-height: 1.35;
}

.advanced-option-danger:has(input:checked) {
  border-color: rgb(239 68 68 / 0.45);
  background: rgb(239 68 68 / 0.06);
}

@media (max-width: 1100px) {
  .settings-core-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}

@media (max-width: 680px) {
  .settings-core-grid { grid-template-columns: minmax(0, 1fr); }
}
</style>

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
