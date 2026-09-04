<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useAppStore } from '@/stores/app'
import { useCompressionProfileStore } from '@/stores/compressionProfile'
import { usePasswordStore, type PasswordEntry } from '@/stores/password'
import type { CompressionOptions } from '@/stores/compression'
import type { CompressionProfile } from '@/types/profile'
import { COMPRESSIBLE_FORMATS, effectiveFormatForPassword, isPasswordSupportedFormat } from '@/utils/compressionFormat'
import ProfileSelector from '@/components/profiles/ProfileSelector.vue'
import ProfileManager from '@/components/profiles/ProfileManager.vue'
import PasswordGeneratorDialog from '@/components/password/PasswordGeneratorDialog.vue'
import { extractErrorMessage } from '@/utils'
import { useArchiveEngine } from '@/composables/useArchiveEngine'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()
const profileStore = useCompressionProfileStore()
const passwordStore = usePasswordStore()
const archiveEngine = useArchiveEngine()

const showPasswordGenerator = ref(false)
const showVaultPasswords = ref(false)
const passwordField = ref<HTMLElement | null>(null)

interface Props {
  modelValue?: CompressionOptions
  outputPath?: string
  allowSingleFileFormats?: boolean
  allowSplitArchive?: boolean
  suggestedFilename?: string
  compact?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  outputPath: '',
  allowSingleFileFormats: true,
  allowSplitArchive: true,
  suggestedFilename: '',
  compact: false
})

interface Emits {
  (e: 'update:modelValue', value: CompressionOptions): void
  (e: 'update:outputPath', value: string): void
  (e: 'template-draft-created'): void
}

const emit = defineEmits<Emits>()

const handleTemplateDraftCreated = () => {
  showProfileSelector.value = false
  emit('template-draft-created')
}

const compressionOptions = ref<CompressionOptions>(props.modelValue || {
  format: 'zip',
  level: 6,
  password: '',
  filename: '',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  verifyAfter: true,
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

const compressionFormats = computed(() => COMPRESSIBLE_FORMATS.filter(format => archiveEngine.canCreate(format.engineFormat)))
const selectedFormat = computed(() => compressionFormats.value.find(format => format.value === compressionOptions.value.format))
const canCreateSplitArchive = computed(() =>
  Boolean(selectedFormat.value?.supportsSplit && props.allowSplitArchive && !compressionOptions.value.password)
)
const vaultPasswordEntries = computed(() => [...passwordStore.entries].sort((left, right) => {
  if (left.favorite !== right.favorite) return left.favorite ? -1 : 1
  if (left.use_count !== right.use_count) return right.use_count - left.use_count
  return left.name.localeCompare(right.name, 'zh-CN', { numeric: true })
}))
const selectedVaultPasswordName = computed(() =>
  passwordStore.entries.find(entry => entry.password === compressionOptions.value.password)?.name || ''
)
const splitArchiveUnavailableReason = computed(() => {
  if (!selectedFormat.value?.supportsSplit) {
    return `${selectedFormat.value?.name || compressionOptions.value.format.toUpperCase()} 格式暂不支持创建分卷`
  }
  if (!props.allowSplitArchive) {
    return '当前任务包含文件夹；分卷 ZIP 目前仅支持普通文件'
  }
  if (compressionOptions.value.password) {
    return '加密分卷 ZIP 暂不支持，请先清除压缩密码'
  }
  return ''
})

const normalizeSplitSize = () => {
  const value = Math.max(1, Math.round(Number(compressionOptions.value.splitSize) || 1024))
  compressionOptions.value.splitSize = String(value)
}

const updateSplitSize = (event: Event) => {
  compressionOptions.value.splitSize = (event.target as HTMLInputElement).value
}

onMounted(() => {
  void archiveEngine.refresh()
  if (!passwordStore.isInitialized) void passwordStore.checkUnlockStatus()
})

// ZIP/7Z/RAR 与 .aes 格式原生支持密码；其他格式显式转为加密 7Z 容器。
const supportsPassword = computed(() => isPasswordSupportedFormat(compressionOptions.value.format))
const usesEncrypted7zContainer = computed(() =>
  Boolean(
    compressionOptions.value.password &&
    compressionOptions.value.format !== '7z' &&
    effectiveFormatForPassword(compressionOptions.value.format, compressionOptions.value.password) === '7z'
  )
)
const effectiveOutputExtension = computed(() =>
  effectiveFormatForPassword(compressionOptions.value.format, compressionOptions.value.password)
)

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

const useSuggestedFilename = () => {
  compressionOptions.value.filename = props.suggestedFilename.trim()
}

watch(compressionOptions, (newOptions) => {
  if (syncingFromProps) return
  emit('update:modelValue', newOptions)
}, { deep: true })

watch(() => compressionOptions.value.format, () => {
  if (!supportsPassword.value) {
    compressionOptions.value.password = ''
  }
  if (!canCreateSplitArchive.value) {
    compressionOptions.value.splitArchive = false
  }
  if (compressionOptions.value.format !== '7z') {
    compressionOptions.value.createSolidArchive = false
  }
})

watch(() => compressionOptions.value.deleteAfter, (deleteAfter) => {
  if (deleteAfter) compressionOptions.value.verifyAfter = true
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
  const currentFormat = compressionFormats.value.find(format => format.value === compressionOptions.value.format)
  if (currentFormat && isFormatDisabled(currentFormat)) {
    compressionOptions.value.format = 'zip'
  }
})

watch(() => props.allowSplitArchive, allowSplitArchive => {
  if (!allowSplitArchive) compressionOptions.value.splitArchive = false
})

watch(() => compressionOptions.value.password, () => {
  if (!canCreateSplitArchive.value) compressionOptions.value.splitArchive = false
})

const applyProfile = (profile: CompressionProfile) => {
  compressionOptions.value.format = profile.config.format as any
  compressionOptions.value.level = profile.config.level
  compressionOptions.value.password = profile.config.password || ''
  compressionOptions.value.splitArchive = profile.config.splitArchive
  compressionOptions.value.splitSize = profile.config.splitSize?.toString() || '1024'
  compressionOptions.value.keepStructure = profile.config.keepStructure
  compressionOptions.value.deleteAfter = profile.config.deleteAfter
  compressionOptions.value.verifyAfter = profile.config.verifyAfter
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
        verifyAfter: compressionOptions.value.verifyAfter,
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

const selectVaultPassword = async (entry: PasswordEntry) => {
  compressionOptions.value.password = await passwordStore.usePassword(entry.id) || entry.password
  showVaultPasswords.value = false
}

const closeVaultPasswordsAfterFocus = () => {
  window.setTimeout(() => {
    if (!passwordField.value?.contains(document.activeElement)) showVaultPasswords.value = false
  }, 0)
}

</script>

<template>
  <div class="horizontal-settings flex min-w-0 max-w-full flex-col gap-4 overflow-x-hidden" :class="{ compact }">
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
        <div class="flex items-center justify-between gap-2 ml-1">
          <label class="text-xs font-black text-muted uppercase tracking-widest">{{ appStore.t('compress.filename') }}</label>
          <button
            type="button"
            class="min-w-0 text-right text-xs font-bold text-primary hover:underline"
            @click="useSuggestedFilename"
          >{{ appStore.t('compress.use_same_name') }}</button>
        </div>
        <div class="relative">
          <input 
            v-model="compressionOptions.filename" 
            class="w-full px-4 py-2 rounded-xl bg-input border border-subtle text-sm text-content outline-none focus:border-primary transition-all placeholder:text-dim"
            :placeholder="appStore.t('vault.placeholder.name')"
          />
          <span class="absolute right-4 top-1/2 -translate-y-1/2 text-xs font-mono text-dim uppercase">.{{ effectiveOutputExtension }}</span>
        </div>
      </div>

      <!-- 密码保护 (主行可见) 带生成器按钮 -->
      <div class="flex flex-col gap-1.5 min-w-0">
        <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('decompress.password') }}</label>
        <div ref="passwordField" class="password-field flex gap-1" @focusout="closeVaultPasswordsAfterFocus">
          <div class="relative min-w-0 flex-1">
            <input
              v-model="compressionOptions.password" type="password"
              data-testid="compression-password-input"
              class="w-full px-3 py-2 rounded-xl bg-input border text-sm outline-none focus:border-primary transition-all disabled:opacity-85 disabled:cursor-not-allowed"
              :class="compressionOptions.password ? 'border-primary/50' : 'border-subtle'"
              :disabled="!supportsPassword"
              :placeholder="supportsPassword ? (compressionOptions.password ? appStore.t('preset.password_set') : appStore.t('preset.password_optional')) : appStore.t('preset.password_na')"
              @focus="showVaultPasswords = supportsPassword"
              @click="showVaultPasswords = supportsPassword"
            />
            <button
              v-if="compressionOptions.password"
              type="button"
              @click="compressionOptions.password = ''; showVaultPasswords = true"
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
          <Transition name="password-menu">
            <div
              v-if="showVaultPasswords && supportsPassword"
              data-testid="compression-password-vault-menu"
              class="password-vault-menu"
            >
              <div class="password-vault-menu-heading">
                <span><i class="pi pi-shield"></i>密码保险箱</span>
                <small>{{ vaultPasswordEntries.length }} 项</small>
              </div>
              <div v-if="passwordStore.isLoading" class="password-vault-menu-state"><i class="pi pi-spinner pi-spin"></i>正在读取密码保险箱</div>
              <div v-else-if="!passwordStore.isUnlocked" class="password-vault-menu-state"><i class="pi pi-lock"></i>密码保险箱当前不可用</div>
              <div v-else-if="vaultPasswordEntries.length === 0" class="password-vault-menu-state"><i class="pi pi-inbox"></i>还没有保存密码</div>
              <div v-else class="password-vault-options custom-scrollbar">
                <button
                  v-for="entry in vaultPasswordEntries"
                  :key="entry.id"
                  type="button"
                  :data-testid="`compression-vault-password-${entry.id}`"
                  :class="{ selected: selectedVaultPasswordName === entry.name }"
                  @click="selectVaultPassword(entry)"
                >
                  <span class="password-vault-option-icon"><i :class="entry.favorite ? 'pi pi-star-fill' : 'pi pi-key'"></i></span>
                  <span><strong>{{ entry.name }}</strong><small>{{ entry.category }}<template v-if="entry.notes"> · {{ entry.notes }}</template></small></span>
                  <i v-if="selectedVaultPasswordName === entry.name" class="pi pi-check"></i>
                </button>
              </div>
            </div>
          </Transition>
        </div>
        <p v-if="usesEncrypted7zContainer" data-testid="encrypted-7z-conversion-hint" class="rounded-lg border border-amber-500/25 bg-amber-500/10 px-2.5 py-2 text-xs leading-5 text-amber-300">
          <i class="pi pi-info-circle mr-1"></i>{{ appStore.t('compress.password_7z_hint') }}
        </p>
      </div>

    </div>

    <div class="flex flex-wrap items-center gap-2">
      <!-- 高级开关按钮 -->
      <button
        data-testid="compression-advanced-options"
        @click="showAdvanced = !showAdvanced"
        class="min-w-0 max-w-full px-4 py-2 rounded-xl border border-subtle text-xs font-black leading-5 transition-all"
        :class="showAdvanced ? 'bg-primary/10 border-primary/30 text-primary' : 'bg-input text-muted hover:text-content'"
      >
        <i class="pi pi-cog mr-2" :class="{ 'animate-spin-slow': showAdvanced }"></i>
        {{ appStore.t('preset.advanced_options', '输出与高级选项') }}
      </button>

      <!-- 配置组选择按钮 -->
      <button
        data-testid="manage-compression-profiles"
        @click="profileDialogMode = 'manage'; showProfileSelector = true"
        class="min-w-0 max-w-full px-4 py-2 rounded-xl border border-subtle text-xs font-black leading-5 transition-all"
        :class="showProfileSelector ? 'bg-sky-500/10 border-sky-500/30 text-sky-400' : 'bg-input text-muted hover:text-content'"
      >
        <i class="pi pi-bookmark mr-2"></i>
        {{ appStore.t('profiles.manage') }}
      </button>

      <!-- 保存为配置组按钮 -->
      <button
        @click="openSaveProfileModal"
        class="min-w-0 max-w-full px-4 py-2 rounded-xl border border-subtle text-xs font-black leading-5 transition-all bg-input text-muted hover:text-content hover:border-sky-500/30"
        :title="appStore.t('profiles.save_as_new')"
      >
        <i class="pi pi-save mr-2"></i>
        {{ appStore.t('profiles.save') }}
      </button>
    </div>

    <section
      class="split-settings-card"
      :class="{ unavailable: !canCreateSplitArchive, enabled: compressionOptions.splitArchive }"
      data-testid="compression-split-settings"
    >
      <label class="split-settings-toggle">
        <input
          v-model="compressionOptions.splitArchive"
          data-testid="compression-split-toggle"
          type="checkbox"
          :disabled="!canCreateSplitArchive"
        />
        <span class="split-settings-icon"><i class="pi pi-clone"></i></span>
        <span class="split-settings-copy">
          <strong>{{ appStore.t('preset.split_archive') }}</strong>
          <small>{{ splitArchiveUnavailableReason || '把大压缩包拆成多个便于传输和存储的 ZIP 分卷' }}</small>
        </span>
      </label>
      <label v-if="compressionOptions.splitArchive" class="split-size-field">
        <span>每卷大小</span>
        <input
          :value="compressionOptions.splitSize"
          data-testid="compression-split-size"
          type="number"
          inputmode="numeric"
          min="1"
          step="1"
          @input="updateSplitSize"
          @blur="normalizeSplitSize"
        />
        <b>MiB</b>
      </label>
      <span v-else-if="canCreateSplitArchive" class="split-settings-state">关闭</span>
    </section>

    <!-- 第二行：高级/路径设置 (条件展开) -->
    <transition name="slide-down">
      <div v-if="showAdvanced" class="space-y-4 pt-4 border-t border-subtle/30">
        <!-- 目标路径 -->
        <div class="flex flex-col gap-1.5 min-w-0">
          <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('compress.output_path') }}</label>
          <div class="flex gap-2">
            <input 
              data-testid="compression-output-path"
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
            <input data-testid="compression-delete-after" v-model="compressionOptions.deleteAfter" type="checkbox" />
            <span><strong>{{ appStore.t('preset.delete_after') }}</strong><small>{{ appStore.t('preset.delete_after.desc') }}</small></span>
          </label>
          <label class="advanced-option" :class="{ 'opacity-70': compressionOptions.deleteAfter }">
            <input data-testid="compression-verify-after" v-model="compressionOptions.verifyAfter" type="checkbox" :disabled="compressionOptions.deleteAfter" />
            <span><strong>{{ appStore.t('preset.verify_after') }}</strong><small>{{ compressionOptions.deleteAfter ? appStore.t('preset.verify_after.required') : appStore.t('preset.verify_after.desc') }}</small></span>
          </label>
          <label v-if="compressionOptions.format === '7z'" class="advanced-option">
            <input v-model="compressionOptions.createSolidArchive" type="checkbox" />
            <span><strong>固实压缩</strong><small>提高同类文件压缩率，但单文件提取更慢</small></span>
          </label>
        </div>
      </div>
    </transition>
  </div>

  <!-- 配置组管理弹窗 -->
  <Teleport to="body">
    <transition name="pop">
      <div v-if="showProfileSelector" class="fixed inset-0 z-[320] flex items-center justify-center bg-black/55 p-4" role="dialog" aria-modal="true" aria-labelledby="profile-dialog-title" @click.self="showProfileSelector = false">
        <div class="modal-no-glass flex max-h-[min(68vh,36rem)] w-full max-w-[min(46rem,calc(100vw-3rem))] flex-col overflow-hidden rounded-[1.4rem] text-content shadow-2xl">
          <div class="flex shrink-0 items-center justify-between gap-4 px-5 pb-3 pt-5">
            <div>
              <h3 id="profile-dialog-title" class="text-base font-black text-content">{{ profileDialogMode === 'select' ? '选择压缩配置' : '管理压缩配置组' }}</h3>
              <p class="text-xs text-muted mt-1">{{ profileDialogMode === 'select' ? '选择后会立即应用格式、压缩级别和高级选项' : '创建、修改或删除可重复使用的配置组' }}</p>
            </div>
            <button type="button" data-testid="close-compression-profiles" class="w-8 h-8 rounded-lg bg-input text-muted hover:text-content" @click="showProfileSelector = false"><i class="pi pi-times"></i></button>
          </div>
          <div class="profile-dialog-scroll custom-scrollbar min-h-0 flex-1 overflow-y-auto px-5 pb-5">
            <ProfileSelector v-if="profileDialogMode === 'select'" :show-manage-button="true" @apply="applyProfile" @manage="profileDialogMode = 'manage'" />
            <div v-else>
              <button type="button" class="mb-4 h-9 px-4 rounded-lg bg-input border border-subtle text-xs font-bold text-muted hover:text-content" @click="profileDialogMode = 'select'"><i class="pi pi-arrow-left mr-2"></i>返回选择配置</button>
              <ProfileManager @draft-created="handleTemplateDraftCreated" />
            </div>
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

.horizontal-settings.compact .settings-core-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.password-field {
  position: relative;
}

.password-vault-menu {
  position: absolute;
  z-index: 80;
  top: calc(100% + 0.42rem);
  right: 0;
  left: 0;
  min-width: min(19rem, calc(100vw - 3rem));
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--dynamic-accent) 28%, var(--border-subtle));
  border-radius: 0.82rem;
  background: var(--bg-modal);
  box-shadow: 0 18px 48px rgb(0 0 0 / 0.32), 0 0 0 1px rgb(255 255 255 / 0.025);
}

.password-vault-menu-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  border-bottom: 1px solid var(--border-subtle);
  padding: 0.58rem 0.7rem;
}

.password-vault-menu-heading span {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  color: var(--text-content);
  font-size: 0.62rem;
  font-weight: 900;
}

.password-vault-menu-heading span i {
  color: var(--dynamic-accent);
  font-size: 0.65rem;
}

.password-vault-menu-heading small {
  color: var(--text-muted);
  font-size: 0.54rem;
  font-weight: 800;
}

.password-vault-options {
  max-height: 11rem;
  overflow-y: auto;
  padding: 0.35rem;
}

.password-vault-options button {
  display: grid;
  width: 100%;
  min-width: 0;
  grid-template-columns: 1.7rem minmax(0, 1fr) 0.8rem;
  align-items: center;
  gap: 0.48rem;
  border-radius: 0.62rem;
  padding: 0.48rem 0.52rem;
  color: var(--text-muted);
  text-align: left;
  transition: background-color 140ms ease, color 140ms ease;
}

.password-vault-options button:hover,
.password-vault-options button.selected {
  background: color-mix(in srgb, var(--dynamic-accent) 10%, transparent);
  color: var(--dynamic-accent);
}

.password-vault-option-icon {
  display: grid;
  width: 1.65rem;
  height: 1.65rem;
  place-items: center;
  border-radius: 0.5rem;
  background: var(--bg-input);
  color: var(--dynamic-accent);
  font-size: 0.58rem;
}

.password-vault-options button > span:nth-child(2) {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.08rem;
}

.password-vault-options strong,
.password-vault-options small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.password-vault-options strong {
  color: var(--text-content);
  font-size: 0.62rem;
  font-weight: 850;
}

.password-vault-options small {
  color: var(--text-muted);
  font-size: 0.52rem;
  font-weight: 650;
}

.password-vault-options button > i {
  font-size: 0.52rem;
}

.password-vault-menu-state {
  display: flex;
  min-height: 4.5rem;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
  padding: 0.75rem;
  color: var(--text-muted);
  font-size: 0.58rem;
  font-weight: 750;
}

.password-menu-enter-active,
.password-menu-leave-active {
  transition: opacity 140ms ease, transform 140ms ease;
}

.password-menu-enter-from,
.password-menu-leave-to {
  opacity: 0;
  transform: translateY(-0.3rem) scale(0.98);
}

.split-settings-card {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 0.75rem;
  border: 1px solid var(--border-subtle);
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--bg-input) 58%, transparent);
  padding: 0.65rem 0.75rem;
  transition: border-color 160ms ease, background-color 160ms ease;
}

.split-settings-card.enabled {
  border-color: color-mix(in srgb, var(--dynamic-accent) 48%, var(--border-subtle));
  background: color-mix(in srgb, var(--dynamic-accent) 7%, var(--bg-input));
}

.split-settings-card.unavailable {
  opacity: 0.72;
}

.split-settings-toggle {
  display: flex;
  min-width: 0;
  flex: 1;
  align-items: center;
  gap: 0.65rem;
  cursor: pointer;
}

.split-settings-toggle:has(input:disabled) {
  cursor: not-allowed;
}

.split-settings-toggle > input {
  width: 1rem;
  height: 1rem;
  flex: 0 0 auto;
  accent-color: var(--dynamic-accent);
}

.split-settings-icon {
  display: grid;
  width: 2rem;
  height: 2rem;
  flex: 0 0 2rem;
  place-items: center;
  border-radius: 0.62rem;
  background: color-mix(in srgb, var(--dynamic-accent) 10%, transparent);
  color: var(--dynamic-accent);
  font-size: 0.72rem;
}

.split-settings-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.12rem;
}

.split-settings-copy strong {
  color: var(--text-base);
  font-size: 0.72rem;
}

.split-settings-copy small {
  overflow: hidden;
  color: var(--text-muted);
  font-size: 0.62rem;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.split-size-field {
  display: grid;
  flex: 0 0 auto;
  grid-template-columns:auto 5.25rem auto;
  align-items: center;
  gap: 0.42rem;
  color: var(--text-muted);
  font-size: 0.62rem;
  font-weight: 800;
}

.split-size-field input {
  width: 5.25rem;
  height: 2rem;
  border: 1px solid var(--border-subtle);
  border-radius: 0.58rem;
  background: var(--bg-card);
  padding: 0 0.55rem;
  color: var(--text-content);
  font-size: 0.68rem;
  font-weight: 800;
  outline: none;
}

.split-size-field input:focus {
  border-color: var(--dynamic-accent);
}

.split-size-field b,
.split-settings-state {
  color: var(--text-muted);
  font-size: 0.58rem;
  font-weight: 850;
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
  .split-settings-card { align-items: stretch; flex-direction: column; }
  .split-size-field { align-self: flex-end; }
  .split-settings-state { display: none; }
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
