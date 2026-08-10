<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { open, save } from '@tauri-apps/api/dialog'
import { useCompressionProfileStore } from '@/stores/compressionProfile'
import { useAppStore } from '@/stores/app'
import type { CompressionProfile, CreateProfileRequest, TaskTemplatePreview } from '@/types/profile'
import { COMPRESSIBLE_FORMATS, FORMAT_CAPABILITIES, isPasswordSupportedFormat } from '@/utils/compressionFormat'
import { extractErrorMessage } from '@/utils'
import Modal from '@/components/ui/Modal.vue'
import { useArchiveEngine } from '@/composables/useArchiveEngine'
import { useCompressionProfiles } from '@/composables/useCompressionProfiles'

const profileStore = useCompressionProfileStore()
const appStore = useAppStore()
const archiveEngine = useArchiveEngine()
const taskTemplates = useCompressionProfiles()

type DialogMode = 'create' | 'edit' | null

const dialogMode = ref<DialogMode>(null)
const editingProfile = ref<CompressionProfile | null>(null)
const deletingProfile = ref<CompressionProfile | null>(null)
const saving = ref(false)
const deleting = ref(false)
const formError = ref('')
const showPassword = ref(false)
const templatePreview = ref<TaskTemplatePreview | null>(null)
const templateFilePath = ref('')
const templateBusy = ref(false)
const exportingProfileId = ref<string | null>(null)

const iconOptions = ['📦', '🗜️', '📁', '🔐', '⚡', '🎯', '💼', '🎨', '🔧', '⭐']
const formatOptions = computed(() => COMPRESSIBLE_FORMATS.filter(format => archiveEngine.canCreate(format.engineFormat)))

const createEmptyForm = (): CreateProfileRequest => ({
  name: '',
  icon: '📦',
  description: '',
  config: {
    format: 'zip',
    level: 6,
    password: null,
    splitArchive: false,
    splitSize: null,
    keepStructure: true,
    deleteAfter: false,
    verifyAfter: true,
    createSolidArchive: false,
    filenameTemplate: null,
    extraParams: {}
  }
})

const formData = ref<CreateProfileRequest>(createEmptyForm())
const profiles = computed(() => profileStore.sortedProfiles)
const loading = computed(() => profileStore.loading)
const selectedCapability = computed(() =>
  FORMAT_CAPABILITIES.find(item => item.format === formData.value.config.format)
)
const supportsPassword = computed(() => isPasswordSupportedFormat(formData.value.config.format))
const supportsSplit = computed(() => selectedCapability.value?.supportsSplit ?? false)
const supportsSolid = computed(() => formData.value.config.format === '7z')
const dialogTitle = computed(() =>
  dialogMode.value === 'create' ? appStore.t('profiles.add_new') : appStore.t('profiles.edit')
)

watch(() => formData.value.config.format, () => {
  if (!supportsPassword.value) formData.value.config.password = null
  if (!supportsSplit.value) {
    formData.value.config.splitArchive = false
    formData.value.config.splitSize = null
  }
  if (!supportsSolid.value) formData.value.config.createSolidArchive = false
})

watch(() => formData.value.config.splitArchive, enabled => {
  if (enabled && !formData.value.config.splitSize) formData.value.config.splitSize = 1024
  if (!enabled) formData.value.config.splitSize = null
})

watch(() => formData.value.config.deleteAfter, enabled => {
  if (enabled) formData.value.config.verifyAfter = true
})

onMounted(async () => {
  void archiveEngine.refresh()
  try {
    await profileStore.loadAllProfiles()
  } catch (error) {
    appStore.setError(extractErrorMessage(error))
  }
})

const openCreateModal = () => {
  formData.value = createEmptyForm()
  editingProfile.value = null
  formError.value = ''
  showPassword.value = false
  dialogMode.value = 'create'
}

const openEditModal = (profile: CompressionProfile) => {
  editingProfile.value = profile
  formData.value = {
    name: profile.name,
    icon: profile.icon,
    description: profile.description,
    config: {
      ...profile.config,
      extraParams: { ...profile.config.extraParams }
    }
  }
  formError.value = ''
  showPassword.value = false
  dialogMode.value = 'edit'
}

const closeFormModal = () => {
  if (saving.value) return
  dialogMode.value = null
  editingProfile.value = null
  formError.value = ''
}

const validateForm = () => {
  const name = formData.value.name.trim()
  if (!name) return appStore.t('profiles.name_required')
  if (name.length > 50) return '配置组名称不能超过 50 个字符'
  if (formData.value.config.splitArchive && (!formData.value.config.splitSize || formData.value.config.splitSize < 1)) {
    return '分卷大小必须大于 0 MB'
  }
  return ''
}

const saveProfile = async () => {
  const error = validateForm()
  if (error) {
    formError.value = error
    return
  }

  saving.value = true
  formError.value = ''
  try {
    const payload: CreateProfileRequest = {
      ...formData.value,
      name: formData.value.name.trim(),
      description: formData.value.description.trim(),
      config: { ...formData.value.config }
    }
    if (dialogMode.value === 'create') {
      await profileStore.addProfile(payload)
    } else if (editingProfile.value) {
      await profileStore.modifyProfile({ ...editingProfile.value, ...payload })
    }
    dialogMode.value = null
    editingProfile.value = null
    appStore.setSuccess(appStore.t('profiles.save_success'))
  } catch (error) {
    formError.value = extractErrorMessage(error)
  } finally {
    saving.value = false
  }
}

const confirmDelete = async () => {
  if (!deletingProfile.value) return
  deleting.value = true
  try {
    await profileStore.removeProfile(deletingProfile.value.id)
    deletingProfile.value = null
    appStore.setSuccess(appStore.t('common.success'))
  } catch (error) {
    appStore.setError(extractErrorMessage(error))
  } finally {
    deleting.value = false
  }
}

const safeTemplateFileName = (name: string) => {
  const normalized = name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '-').replace(/[. ]+$/g, '').trim()
  return `${normalized || 'task-template'}.longtask.json`
}

const exportTemplate = async (profile: CompressionProfile) => {
  const filePath = await save({
    title: '导出任务模板',
    defaultPath: safeTemplateFileName(profile.name),
    filters: [{ name: 'Long解压任务模板', extensions: ['json'] }],
  })
  if (!filePath) return

  exportingProfileId.value = profile.id
  try {
    await taskTemplates.exportTaskTemplate(profile.id, filePath)
    appStore.setSuccess('任务模板已安全导出，不包含固定密码与删除源文件设置')
  } catch (error) {
    appStore.setError(extractErrorMessage(error))
  } finally {
    exportingProfileId.value = null
  }
}

const selectTemplateForImport = async () => {
  const selection = await open({
    title: '选择任务模板',
    multiple: false,
    filters: [{ name: 'Long解压任务模板', extensions: ['json'] }],
  })
  if (typeof selection !== 'string') return

  templateBusy.value = true
  try {
    templatePreview.value = await taskTemplates.previewTaskTemplate(selection)
    templateFilePath.value = selection
  } catch (error) {
    appStore.setError(extractErrorMessage(error))
  } finally {
    templateBusy.value = false
  }
}

const closeTemplatePreview = () => {
  if (templateBusy.value) return
  templatePreview.value = null
  templateFilePath.value = ''
}

const confirmTemplateImport = async () => {
  if (!templatePreview.value || !templateFilePath.value) return
  templateBusy.value = true
  try {
    await taskTemplates.importTaskTemplate(
      templateFilePath.value,
      templatePreview.value.contentSha256,
    )
    await profileStore.loadAllProfiles()
    templatePreview.value = null
    templateFilePath.value = ''
    appStore.setSuccess('任务模板已导入为配置组，尚未执行任何压缩任务')
  } catch (error) {
    appStore.setError(extractErrorMessage(error))
  } finally {
    templateBusy.value = false
  }
}

const passwordStrategyLabel = computed(() => {
  const strategy = templatePreview.value?.template.passwordStrategy
  if (!strategy || strategy.mode === 'none') return '不使用密码'
  if (strategy.mode === 'prompt_at_runtime') return '执行时询问密码'
  if (strategy.mode === 'from_vault') return '执行时从密码保险箱选择'
  return `执行时自动生成 ${strategy.length} 位密码`
})

const sourceRuleLabel = computed(() => {
  const rules = templatePreview.value?.template.sourceRules
  if (!rules || rules.mode === 'manual_selection') return '每次手动选择文件'
  if (rules.mode === 'all') return '建议匹配全部文件（导入后默认停用）'
  if (rules.mode === 'pattern') return `建议匹配：${rules.includePatterns.join('、')}`
  return rules.sizeRangeMib
    ? `建议大小：${rules.sizeRangeMib[0]}–${rules.sizeRangeMib[1]} MiB`
    : '按文件大小建议匹配'
})

const targetRuleLabel = computed(() =>
  templatePreview.value?.template.targetRule.mode === 'same_directory'
    ? '源文件同目录（执行前仍需确认）'
    : '执行时选择目录'
)

const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  return `${(bytes / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`
}

const formatDate = (timestamp: number | null) =>
  timestamp ? new Date(timestamp).toLocaleDateString() : '从未使用'
</script>

<template>
  <div class="profile-manager min-w-0">
    <div class="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-subtle bg-input/25 px-4 py-3">
      <div class="min-w-0">
        <p class="text-sm font-black text-content">{{ profiles.length }} 个配置组</p>
        <p class="mt-0.5 text-xs text-muted">保存常用格式、压缩等级、加密和分卷设置</p>
      </div>
      <div class="flex min-w-0 flex-wrap justify-end gap-2">
        <button data-testid="import-task-template" type="button" class="h-9 shrink-0 rounded-lg border border-primary/30 bg-primary/10 px-4 text-xs font-black text-primary transition hover:bg-primary/15 disabled:cursor-wait disabled:opacity-60" :disabled="templateBusy" @click="selectTemplateForImport">
          <i :class="templateBusy ? 'pi pi-spin pi-spinner' : 'pi pi-file-import'" class="mr-2"></i>导入任务模板
        </button>
        <button type="button" class="h-9 shrink-0 rounded-lg bg-primary px-4 text-xs font-black text-white shadow-lg shadow-primary/20 transition hover:brightness-110" @click="openCreateModal">
          <i class="pi pi-plus mr-2"></i>{{ appStore.t('profiles.add_new') }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="flex min-h-48 items-center justify-center text-primary">
      <i class="pi pi-spin pi-spinner text-xl"></i>
    </div>

    <div v-else-if="profiles.length === 0" class="flex min-h-52 flex-col items-center justify-center rounded-2xl border border-dashed border-subtle bg-input/15 px-6 text-center">
      <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-primary/10 text-2xl">📦</div>
      <p class="font-black text-content">还没有配置组</p>
      <p class="mt-1 max-w-sm text-xs leading-5 text-muted">创建后可以一键恢复常用的压缩格式和高级选项。</p>
      <button type="button" class="mt-4 h-9 rounded-lg border border-primary/30 bg-primary/10 px-4 text-xs font-black text-primary hover:bg-primary/15" @click="openCreateModal">
        {{ appStore.t('profiles.add_new') }}
      </button>
    </div>

    <div v-else class="grid grid-cols-1 gap-3 md:grid-cols-2">
      <article v-for="profile in profiles" :key="profile.id" class="group min-w-0 rounded-2xl border border-subtle bg-card/60 p-4 transition hover:border-primary/35 hover:bg-card">
        <div class="flex min-w-0 items-start gap-3">
          <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-subtle bg-input/40 text-2xl">{{ profile.icon }}</div>
          <div class="min-w-0 flex-1">
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0">
                <h4 class="truncate text-sm font-black text-content">{{ profile.name }}</h4>
                <p class="mt-1 line-clamp-2 min-h-8 text-xs leading-4 text-muted">{{ profile.description || '未填写说明' }}</p>
              </div>
              <div class="flex shrink-0 gap-1">
                <button :data-testid="`export-task-template-${profile.id}`" type="button" class="h-8 w-8 rounded-lg text-muted hover:bg-primary/10 hover:text-primary disabled:cursor-wait disabled:opacity-60" title="导出安全任务模板" :disabled="exportingProfileId === profile.id" @click="exportTemplate(profile)"><i :class="exportingProfileId === profile.id ? 'pi pi-spin pi-spinner' : 'pi pi-file-export'" class="text-xs"></i></button>
                <button type="button" class="h-8 w-8 rounded-lg text-muted hover:bg-primary/10 hover:text-primary" :title="appStore.t('profiles.edit')" @click="openEditModal(profile)"><i class="pi pi-pencil text-xs"></i></button>
                <button type="button" class="h-8 w-8 rounded-lg text-muted hover:bg-red-500/10 hover:text-red-400" :title="appStore.t('profiles.delete')" @click="deletingProfile = profile"><i class="pi pi-trash text-xs"></i></button>
              </div>
            </div>

            <div class="mt-3 flex flex-wrap gap-1.5 text-xs font-bold">
              <span class="rounded-md bg-primary/10 px-2 py-1 text-primary">{{ profile.config.format.toUpperCase() }}</span>
              <span class="rounded-md bg-input px-2 py-1 text-muted">等级 {{ profile.config.level }}</span>
              <span v-if="profile.config.password" class="rounded-md bg-amber-500/10 px-2 py-1 text-amber-400"><i class="pi pi-lock mr-1"></i>加密</span>
              <span v-if="profile.config.splitArchive" class="rounded-md bg-violet-500/10 px-2 py-1 text-violet-400">分卷</span>
              <span v-if="profile.config.createSolidArchive" class="rounded-md bg-emerald-500/10 px-2 py-1 text-emerald-400">固实</span>
            </div>
          </div>
        </div>

        <div class="mt-4 grid grid-cols-3 gap-2 border-t border-subtle/60 pt-3 text-center">
          <div><p class="text-xs font-black text-content">{{ profile.stats.useCount }}</p><p class="text-xs text-muted">使用次数</p></div>
          <div><p class="text-xs font-black text-content">{{ formatBytes(profile.stats.totalBytesProcessed) }}</p><p class="text-xs text-muted">处理量</p></div>
          <div><p class="truncate text-xs font-black text-content">{{ formatDate(profile.lastUsedAt) }}</p><p class="text-xs text-muted">最近使用</p></div>
        </div>
      </article>
    </div>

    <Modal
      :visible="dialogMode !== null"
      :title="dialogTitle"
      description="设置保存后可在压缩中心快速应用"
      icon="pi pi-sliders-h"
      size="lg"
      layer="nested"
      :close-on-backdrop="!saving"
      :close-on-escape="!saving"
      @update:visible="value => { if (!value) closeFormModal() }"
    >
      <form class="space-y-5" @submit.prevent="saveProfile">
        <div>
          <label class="mb-2 block text-xs font-black text-muted">图标</label>
          <div class="flex flex-wrap gap-2">
            <button v-for="icon in iconOptions" :key="icon" type="button" class="h-10 w-10 rounded-lg border text-xl transition" :class="formData.icon === icon ? 'border-primary bg-primary/15 ring-2 ring-primary/20' : 'border-subtle bg-input hover:border-primary/40'" @click="formData.icon = icon">{{ icon }}</button>
          </div>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <label class="block min-w-0">
            <span class="mb-2 block text-xs font-black text-muted">配置组名称 *</span>
            <input v-model="formData.name" maxlength="50" autofocus class="h-10 w-full rounded-xl border border-subtle bg-input px-3 text-sm text-content outline-none focus:border-primary" placeholder="例如：日常 ZIP、最大压缩" />
          </label>
          <label class="block min-w-0">
            <span class="mb-2 block text-xs font-black text-muted">压缩格式</span>
            <select v-model="formData.config.format" class="h-10 w-full rounded-xl border border-subtle bg-input px-3 text-sm text-content outline-none focus:border-primary">
              <option v-for="format in formatOptions" :key="format.value" :value="format.value">{{ format.name }}</option>
            </select>
          </label>
        </div>

        <label class="block">
          <span class="mb-2 block text-xs font-black text-muted">用途说明（可选）</span>
          <textarea v-model="formData.description" rows="2" maxlength="160" class="w-full resize-none rounded-xl border border-subtle bg-input px-3 py-2 text-sm text-content outline-none focus:border-primary" placeholder="说明这个配置组适合什么场景"></textarea>
        </label>

        <div class="rounded-2xl border border-subtle bg-input/20 p-4">
          <div class="mb-3 flex items-center justify-between gap-3">
            <div><p class="text-xs font-black text-content">压缩等级</p><p class="mt-0.5 text-xs text-muted">数值越高通常压缩率越高，但耗时更长</p></div>
            <span class="rounded-lg bg-primary/10 px-3 py-1 text-sm font-black text-primary">{{ formData.config.level }}</span>
          </div>
          <input v-model.number="formData.config.level" type="range" min="0" max="9" class="w-full accent-primary" />
          <div class="mt-1 flex justify-between text-xs text-muted"><span>0 · 仅打包</span><span>9 · 最大压缩</span></div>
        </div>

        <label v-if="supportsPassword" class="block">
          <span class="mb-2 block text-xs font-black text-muted">固定密码（可选）</span>
          <div class="flex gap-2">
            <input v-model="formData.config.password" :type="showPassword ? 'text' : 'password'" class="h-10 min-w-0 flex-1 rounded-xl border border-subtle bg-input px-3 font-mono text-sm text-content outline-none focus:border-primary" placeholder="留空表示不加密" />
            <button type="button" class="h-10 w-10 shrink-0 rounded-xl border border-subtle bg-input text-muted hover:text-content" @click="showPassword = !showPassword"><i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye'"></i></button>
          </div>
        </label>

        <div class="grid gap-3 md:grid-cols-2">
          <label class="profile-option"><input v-model="formData.config.keepStructure" type="checkbox" /><span><strong>保留目录结构</strong><small>归档内保留原始文件夹层级</small></span></label>
          <label class="profile-option"><input v-model="formData.config.deleteAfter" type="checkbox" /><span><strong>完成后删除源文件</strong><small>仅在完整性校验通过后执行</small></span></label>
          <label class="profile-option" :class="{ 'opacity-70': formData.config.deleteAfter }"><input v-model="formData.config.verifyAfter" type="checkbox" :disabled="formData.config.deleteAfter" /><span><strong>压缩完成后校验</strong><small>{{ formData.config.deleteAfter ? '删除源文件时强制开启' : '发布最终文件前读取并验证归档' }}</small></span></label>
          <label v-if="supportsSplit" class="profile-option"><input v-model="formData.config.splitArchive" type="checkbox" /><span><strong>分卷压缩</strong><small>将压缩包拆分为多个文件</small></span></label>
          <label v-if="supportsSolid" class="profile-option"><input v-model="formData.config.createSolidArchive" type="checkbox" /><span><strong>固实压缩</strong><small>提高同类文件压缩率</small></span></label>
        </div>

        <label v-if="formData.config.splitArchive" class="block">
          <span class="mb-2 block text-xs font-black text-muted">每个分卷大小（MB）</span>
          <input v-model.number="formData.config.splitSize" type="number" min="1" step="1" class="h-10 w-full rounded-xl border border-subtle bg-input px-3 text-sm text-content outline-none focus:border-primary" />
        </label>

        <p v-if="formError" role="alert" class="rounded-xl border border-red-500/25 bg-red-500/10 px-3 py-2 text-xs font-bold text-red-400"><i class="pi pi-exclamation-circle mr-2"></i>{{ formError }}</p>

        <div class="flex justify-end gap-2 border-t border-subtle pt-4">
          <button type="button" class="h-10 rounded-xl border border-subtle bg-input px-5 text-xs font-black text-muted hover:text-content" :disabled="saving" @click="closeFormModal">取消</button>
          <button type="submit" class="h-10 min-w-28 rounded-xl bg-primary px-5 text-xs font-black text-white shadow-lg shadow-primary/20 hover:brightness-110 disabled:cursor-wait disabled:opacity-60" :disabled="saving">
            <i v-if="saving" class="pi pi-spin pi-spinner mr-2"></i>{{ saving ? '正在保存' : '保存配置组' }}
          </button>
        </div>
      </form>
    </Modal>

    <Modal
      :visible="Boolean(templatePreview)"
      title="导入任务模板"
      description="审计预览 · 确认后只创建配置组"
      icon="pi pi-shield"
      size="lg"
      layer="nested"
      :close-on-backdrop="!templateBusy"
      :close-on-escape="!templateBusy"
      @update:visible="value => { if (!value) closeTemplatePreview() }"
    >
      <div v-if="templatePreview" data-testid="task-template-preview" class="min-w-0 space-y-4 overflow-x-hidden">
        <div class="rounded-2xl border border-emerald-500/25 bg-emerald-500/10 p-4">
          <div class="flex items-start gap-3">
            <i class="pi pi-check-circle mt-0.5 shrink-0 text-emerald-400"></i>
            <div class="min-w-0">
              <p class="text-sm font-black text-content">安全边界已应用</p>
              <p class="mt-1 text-xs leading-5 text-muted">模板不携带固定密码、删除源文件和额外引擎参数；导入不会启动压缩，自动匹配规则默认停用。</p>
            </div>
          </div>
        </div>

        <div class="grid min-w-0 gap-3 sm:grid-cols-2">
          <section class="min-w-0 rounded-2xl border border-subtle bg-input/25 p-4">
            <p class="text-xs font-black uppercase tracking-wider text-muted">模板身份</p>
            <div class="mt-3 flex min-w-0 items-center gap-3">
              <span class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-xl">{{ templatePreview.template.icon }}</span>
              <div class="min-w-0">
                <p class="truncate text-sm font-black text-content">{{ templatePreview.template.name }}</p>
                <p class="mt-0.5 text-xs text-muted">结构版本 {{ templatePreview.template.version }}</p>
              </div>
            </div>
            <p class="mt-3 break-words text-xs leading-5 text-muted">{{ templatePreview.template.description || '未填写用途说明' }}</p>
          </section>

          <section class="min-w-0 rounded-2xl border border-subtle bg-input/25 p-4">
            <p class="text-xs font-black uppercase tracking-wider text-muted">压缩方案</p>
            <div class="mt-3 flex flex-wrap gap-1.5 text-xs font-bold">
              <span class="rounded-md bg-primary/10 px-2 py-1 text-primary">{{ templatePreview.template.compression.format.toUpperCase() }}</span>
              <span class="rounded-md bg-card px-2 py-1 text-muted">等级 {{ templatePreview.template.compression.level }}</span>
              <span v-if="templatePreview.template.compression.splitArchive" class="rounded-md bg-violet-500/10 px-2 py-1 text-violet-400">分卷</span>
              <span v-if="templatePreview.template.compression.createSolidArchive" class="rounded-md bg-emerald-500/10 px-2 py-1 text-emerald-400">固实</span>
            </div>
            <dl class="mt-3 space-y-2 text-xs">
              <div class="flex min-w-0 justify-between gap-3"><dt class="shrink-0 text-muted">完整性校验</dt><dd class="min-w-0 text-right font-bold text-content">{{ templatePreview.template.compression.verifyAfter ? '开启' : '关闭' }}</dd></div>
              <div class="flex min-w-0 justify-between gap-3"><dt class="shrink-0 text-muted">目录结构</dt><dd class="min-w-0 text-right font-bold text-content">{{ templatePreview.template.compression.keepStructure ? '保留' : '不保留' }}</dd></div>
            </dl>
          </section>
        </div>

        <section class="min-w-0 rounded-2xl border border-subtle bg-input/25 p-4">
          <p class="text-xs font-black uppercase tracking-wider text-muted">运行时计划</p>
          <dl class="mt-3 grid min-w-0 gap-3 text-xs sm:grid-cols-2">
            <div class="min-w-0"><dt class="text-muted">源文件</dt><dd class="mt-1 break-words font-bold leading-5 text-content">{{ sourceRuleLabel }}</dd></div>
            <div class="min-w-0"><dt class="text-muted">输出位置</dt><dd class="mt-1 break-words font-bold leading-5 text-content">{{ targetRuleLabel }}</dd></div>
            <div class="min-w-0"><dt class="text-muted">压缩包名称</dt><dd class="mt-1 break-all font-mono font-bold leading-5 text-content">{{ templatePreview.template.targetRule.filenameTemplate || '使用源文件同名压缩包' }}</dd></div>
            <div class="min-w-0"><dt class="text-muted">密码策略</dt><dd class="mt-1 break-words font-bold leading-5 text-content">{{ passwordStrategyLabel }}</dd></div>
          </dl>
        </section>

        <section v-if="templatePreview.warnings.length" class="min-w-0 rounded-2xl border border-amber-500/25 bg-amber-500/10 p-4">
          <p class="text-xs font-black text-amber-400"><i class="pi pi-exclamation-triangle mr-2"></i>导入前提醒</p>
          <ul class="mt-2 space-y-1.5 pl-5 text-xs leading-5 text-muted">
            <li v-for="warning in templatePreview.warnings" :key="warning" class="list-disc break-words">{{ warning }}</li>
          </ul>
        </section>

        <p class="break-all rounded-xl border border-subtle bg-input/20 px-3 py-2 font-mono text-[10px] leading-4 text-muted">SHA-256 {{ templatePreview.contentSha256 }}</p>

        <div class="flex flex-col-reverse gap-2 border-t border-subtle pt-4 sm:flex-row sm:justify-end">
          <button type="button" class="h-10 rounded-xl border border-subtle bg-input px-5 text-xs font-black text-muted hover:text-content" :disabled="templateBusy" @click="closeTemplatePreview">取消</button>
          <button data-testid="confirm-task-template-import" type="button" class="h-10 rounded-xl bg-primary px-5 text-xs font-black text-white shadow-lg shadow-primary/20 hover:brightness-110 disabled:cursor-wait disabled:opacity-60" :disabled="templateBusy" @click="confirmTemplateImport">
            <i v-if="templateBusy" class="pi pi-spin pi-spinner mr-2"></i>{{ templateBusy ? '正在导入' : '确认导入配置组（不执行）' }}
          </button>
        </div>
      </div>
    </Modal>

    <Modal
      :visible="Boolean(deletingProfile)"
      title="删除配置组"
      :description="deletingProfile?.name"
      icon="pi pi-trash"
      size="sm"
      layer="nested"
      :close-on-backdrop="!deleting"
      @update:visible="value => { if (!value && !deleting) deletingProfile = null }"
    >
      <p class="text-sm leading-6 text-muted">删除后无法恢复，但不会影响已经创建的压缩任务。</p>
      <div class="mt-5 flex justify-end gap-2">
        <button type="button" class="h-10 rounded-xl border border-subtle bg-input px-5 text-xs font-black text-muted" :disabled="deleting" @click="deletingProfile = null">取消</button>
        <button type="button" class="h-10 rounded-xl bg-red-500 px-5 text-xs font-black text-white disabled:opacity-60" :disabled="deleting" @click="confirmDelete"><i v-if="deleting" class="pi pi-spin pi-spinner mr-2"></i>确认删除</button>
      </div>
    </Modal>
  </div>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}

.profile-option {
  display: flex;
  min-width: 0;
  cursor: pointer;
  align-items: flex-start;
  gap: 0.75rem;
  border: 1px solid var(--border-subtle);
  border-radius: 0.875rem;
  padding: 0.75rem;
  background: color-mix(in srgb, var(--bg-input) 55%, transparent);
}

.profile-option input {
  margin-top: 0.125rem;
  width: 1rem;
  height: 1rem;
  accent-color: var(--dynamic-accent);
}

.profile-option span { min-width: 0; }
.profile-option strong { display: block; color: var(--text-base); font-size: 0.75rem; }
.profile-option small { display: block; margin-top: 0.2rem; color: var(--text-muted); font-size: 0.7rem; line-height: 1rem; }
</style>
