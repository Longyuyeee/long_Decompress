<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useArchiveEngine } from '@/composables/useArchiveEngine'
import { FORMAT_CAPABILITIES, EXTRACT_ONLY_FORMATS, type CompressionFormatCapability } from '@/utils/compressionFormat'
import { APP_THEMES } from '@/utils/themes'
import AccessibilitySettings from '@/components/settings/AccessibilitySettings.vue'
import WorkspaceHeader from '@/components/ui/WorkspaceHeader.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import { useUpdateStore } from '@/stores/update'

const appStore = useAppStore()
const updateStore = useUpdateStore()
const tauriCommands = useTauriCommands()
const { capabilities: archiveEngine, loading: archiveEngineLoading, refresh: refreshArchiveEngine } = useArchiveEngine()
const showResetConfirm = ref(false)
const settingsSection = ref('appearance')
const settingsScroll = ref<HTMLElement>()
const settingsSections = [
  { id:'appearance', label:'外观', icon:'pi pi-palette' },
  { id:'accessibility', label:'辅助显示', icon:'pi pi-eye' },
  { id:'processing', label:'压缩与解压', icon:'pi pi-sliders-h' },
  { id:'behavior', label:'行为', icon:'pi pi-cog' },
  { id:'updates', label:'软件更新', icon:'pi pi-refresh' },
  { id:'formats', label:'格式支持', icon:'pi pi-box' },
]
const selectSettings = (section: string) => { settingsSection.value = section; settingsScroll.value?.scrollTo?.({ top:0 }) }

const rarEncoder = ref<{ available: boolean; message: string } | null>(null)
const currentVersion = ref('—')
const diagnosticsLoading = computed(() => archiveEngineLoading.value)
const engineSupports = (format: CompressionFormatCapability, create = false) => !format.engineFormat || archiveEngine.value?.formats.some(item =>
  (!create || item.canCreate) && [item.name, ...item.extensions].some(value => value.toLowerCase() === format.engineFormat?.toLowerCase()))
const readableFormats = computed(() => FORMAT_CAPABILITIES.filter(format => format.canExtract && engineSupports(format) && (!format.requires7za || archiveEngine.value?.available)))
const creatableFormats = computed(() => FORMAT_CAPABILITIES.filter(format => format.canCompress && engineSupports(format, true) && (!format.requiresWinRar || rarEncoder.value?.available)))
const readableExtensions = computed(() => [...new Set([
  ...readableFormats.value.flatMap(format => format.extensions),
  ...EXTRACT_ONLY_FORMATS.filter(format => {
    if (['ALZ', 'EGG'].includes(format.displayName)) return archiveEngine.value?.formats.some(item => item.name === format.displayName)
    return !format.requires7za || archiveEngine.value?.available
  }).flatMap(format => format.extensions),
  ...(archiveEngine.value?.formats.flatMap(format => format.extensions) || []),
].map(extension => extension.toLowerCase()))].sort())
const readableExtensionCount = computed(() => readableExtensions.value.length)
const passwordCompressionFormats = computed(() => creatableFormats.value
  .filter(format => format.supportsPasswordCompress)
  .map(format => format.displayName)
  .join(' · '))
const passwordExtractionNames = computed(() => readableFormats.value
  .filter(format => format.supportsPasswordExtract)
  .map(format => format.displayName)
  .concat(archiveEngine.value?.formats.some(item => item.name === 'ALZ') ? ['ALZ（只读）'] : [])
  .concat(archiveEngine.value?.formats.some(item => item.name === 'EGG') ? ['EGG（只读）'] : []))
const passwordExtractionFormats = computed(() => passwordExtractionNames.value.join(' · '))

const refreshEngineDiagnostics = async () => {
  await refreshArchiveEngine()
  try {
    rarEncoder.value = await tauriCommands.checkRarCompressionSupport()
  } catch (error) {
    rarEncoder.value = { available: false, message: String(error) }
  }
}

const themeColors = {
  azure: '#0ea5e9', indigo: '#6366f1', violet: '#8b5cf6',
  fuchsia: '#d946ef', pink: '#ec4899', rose: '#f43f5e',
  orange: '#f97316', amber: '#f59e0b', lime: '#84cc16',
  emerald: '#10b981', teal: '#14b8a6', cyan: '#06b6d4',
  slate: '#64748b'
}

const themeModes = APP_THEMES
const selectTheme = (mode: typeof APP_THEMES[number]) => {
  appStore.theme = mode.value
  if ('accent' in mode) appStore.accentColor = mode.accent
  appStore.saveSettingsToStorage()
}

const selectedAccentName = computed(() =>
  Object.entries(themeColors).find(([, hex]) => hex === appStore.accentColor)?.[0] || 'custom'
)

const contextMenuBusy = ref(false)
const autoStartBusy = ref(false)
const autoStartSupported = ref(navigator.platform.toLowerCase().includes('win'))
const toggleBruteForce = () => appStore.updateSettings({ enableBruteForce: !appStore.settings.enableBruteForce })

const contextMenuSupported = ref(navigator.platform.toLowerCase().includes('win'))

// 验证并更新线程数
const validateAndUpdateThreads = (value: number) => {
  const validated = Math.max(1, Math.min(16, Math.floor(value)))
  appStore.updateSettings({ maxConcurrentTasks: validated })
  appStore.saveSettingsToStorage()
}

// 验证并更新 UI 缩放
const validateAndUpdateUIScale = (value: number) => {
  const validated = Math.max(60, Math.min(200, Math.floor(value)))
  appStore.updateSettings({ uiScale: validated })
  appStore.saveSettingsToStorage()
}

const checkContextMenu = async () => {
  if (!contextMenuSupported.value) return
  try {
    await appStore.synchronizeContextMenu()
  } catch { /* ignore */ }
}

const checkAutoStart = async () => {
  if (!autoStartSupported.value) return
  try {
    const enabled = await tauriCommands.invoke<boolean>('check_auto_start')
    if (appStore.settings.autoStart !== enabled) {
      appStore.updateSettings({ autoStart: enabled })
    }
  } catch {
    autoStartSupported.value = false
  }
}

const toggleAutoStart = async () => {
  if (!autoStartSupported.value || autoStartBusy.value) return
  autoStartBusy.value = true
  const enable = !appStore.settings.autoStart
  try {
    const registered = await tauriCommands.invoke<boolean>('set_auto_start', { enable })
    if (registered !== enable) {
      throw new Error(appStore.t('settings.performance.auto_start.verify_failed'))
    }
    appStore.updateSettings({ autoStart: registered })
    appStore.setSuccess(appStore.t(
      registered
        ? 'settings.performance.auto_start.enabled'
        : 'settings.performance.auto_start.disabled'
    ))
  } catch (error) {
    appStore.setError(String(error))
    await checkAutoStart()
  } finally {
    autoStartBusy.value = false
  }
}

onMounted(async () => {
  void checkAutoStart()
  checkContextMenu()
  void refreshEngineDiagnostics()
  try {
    currentVersion.value = await getVersion()
  } catch { /* browser preview */ }
})

const toggleAutoCheckUpdates = () => {
  const enabled = !appStore.settings.autoCheckUpdates
  appStore.updateSettings({ autoCheckUpdates: enabled })
  updateStore.scheduleAutoCheck(enabled)
}

const checkForUpdatesNow = () => updateStore.checkForUpdates(true)
const formatUpdateTime = (value: number | null) => {
  if (!value) return appStore.t('settings.update.never')
  return new Intl.DateTimeFormat(appStore.language === 'en-US' ? 'en-US' : 'zh-CN', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value))
}
const onlineVersion = computed(() => {
  if (updateStore.availableVersion) return `v${updateStore.availableVersion}`
  if (updateStore.status === 'up-to-date' && currentVersion.value !== '—') return `v${currentVersion.value}`
  return appStore.t('settings.update.unknown')
})
const updateStatusText = computed(() => appStore.t(`settings.update.status.${updateStore.status}`))
const toggleContextMenu = async () => {
  if (contextMenuBusy.value) return
  contextMenuBusy.value = true
  const enable = !appStore.settings.contextMenuEnabled
  try {
    await appStore.setContextMenuEnabled(enable)
    appStore.setSuccess(appStore.t('common.success'))
  } catch (e: any) {
    appStore.setError(String(e))
  } finally {
    contextMenuBusy.value = false
  }
}

const addWordlist = async () => {
  const paths = await tauriCommands.selectWordlists()
  if (paths.length > 0) {
    const current = new Set(appStore.settings.bruteForceWordlists)
    paths.forEach(p => current.add(p))
    appStore.updateSettings({ bruteForceWordlists: Array.from(current) })
    appStore.setSuccess(appStore.t('common.success'))
  }
}

const removeWordlist = (index: number) => {
  const newList = [...appStore.settings.bruteForceWordlists]
  newList.splice(index, 1)
  appStore.updateSettings({ bruteForceWordlists: newList })
}
</script>

<template>
  <div class="settings-view flex flex-col transition-colors duration-700 overflow-hidden">
    <WorkspaceHeader :title="appStore.t('settings.title')" icon="pi pi-cog">
      <button class="settings-reset-action" @click="showResetConfirm=true"><i class="pi pi-replay"/>恢复默认</button>
      <template #navigation><nav class="module-tabs settings-tabs" aria-label="设置分类"><button v-for="section in settingsSections" :key="section.id" :data-settings-section="section.id" :class="{active:settingsSection===section.id}" :aria-pressed="settingsSection===section.id" @click="selectSettings(section.id)"><i :class="section.icon"/>{{section.label}}</button></nav></template>
    </WorkspaceHeader>

    <div ref="settingsScroll" class="settings-scroll flex-1 overflow-y-auto custom-scrollbar">
      <div class="settings-content">
        <!-- 外观：标签与控件对齐，窄窗口自动单列 -->
        <section v-show="settingsSection==='appearance'" data-settings-panel="appearance" class="aero-card settings-appearance">
          <div class="settings-group">
            <div class="settings-group-heading">
              <div class="flex items-center gap-2"><h2 class="text-base font-black text-content uppercase tracking-widest">{{ appStore.t('settings.appearance') }}</h2><HelpHint :label="appStore.t('settings.appearance')">{{ appStore.t('settings.appearance.desc') }}</HelpHint></div>
            </div>
            
            <div class="settings-group-body">
              <!-- 模式切换 -->
              <div class="settings-option-row settings-theme-row">
                <label class="text-xs font-black text-muted uppercase tracking-[0.2em] block ml-1">{{ appStore.t('settings.theme') }}</label>
                <div class="settings-theme-options">
                  <button
                    v-for="m in themeModes" :key="m.value"
                    :aria-pressed="appStore.theme === m.value"
                    :data-theme="m.value"
                    :title="'accent' in m ? appStore.t(`settings.theme.desc.${m.value}`) : undefined"
                    :style="{ '--preview-bg': m.preview[0], '--preview-panel': m.preview[1], '--preview-accent': m.preview[2] }"
                    @click="selectTheme(m)"
                    class="settings-theme-card"
                    :class="appStore.theme === m.value ? 'bg-primary text-white shadow-lg' : 'text-muted hover:bg-white/5'"
                  >
                    <span class="settings-theme-preview" aria-hidden="true"><span></span><span><b></b><b></b><b></b></span></span>
                    <span class="settings-theme-name"><i :class="m.icon" aria-hidden="true"></i>{{ appStore.t(`settings.theme.${m.value}`) }}<i v-if="appStore.theme === m.value" class="pi pi-check" aria-hidden="true"></i></span>
                  </button>
                </div>
              </div>

              <!-- 强调色选择 -->
              <div class="settings-option-row">
                <div class="settings-accent-label">
                  <label class="text-xs font-black text-muted uppercase tracking-[0.2em]">{{ appStore.t('settings.accent') }}</label>
                  <span class="rounded-lg border border-primary/30 bg-primary/10 px-2.5 py-1 text-xs font-black uppercase tracking-wider text-primary">{{ selectedAccentName }}</span>
                </div>
                <div class="settings-accent-options flex flex-wrap">
                  <button 
                    v-for="(hex, name) in themeColors" :key="name"
                    @click="appStore.accentColor = hex; appStore.saveSettingsToStorage()"
                    class="relative flex h-9 w-9 items-center justify-center rounded-xl border-2 transition-all hover:scale-110"
                    :class="appStore.accentColor === hex ? 'scale-110 border-content shadow-lg ring-2 ring-primary/35 ring-offset-2 ring-offset-card' : 'border-transparent opacity-75 hover:opacity-100'"
                    :style="{ backgroundColor: hex }"
                    :aria-pressed="appStore.accentColor === hex"
                    :title="name"
                  ><i v-if="appStore.accentColor === hex" class="pi pi-check text-xs font-black text-white drop-shadow-[0_1px_2px_rgba(0,0,0,.8)]"></i></button>
                </div>
              </div>

              <!-- 语言选择 -->
              <div class="settings-option-row settings-language border-t border-subtle">
                <span class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.language') }}</span>
                <div class="flex flex-wrap gap-2">
                  <button @click="appStore.language = 'zh-CN'; appStore.saveSettingsToStorage()" 
                          class="px-4 py-1.5 rounded-lg text-sm font-black transition-all border border-subtle"
                          :class="appStore.language === 'zh-CN' ? 'bg-primary text-white border-primary' : 'bg-input text-muted'">{{ appStore.t('settings.lang.zh') }}</button>
                  <button @click="appStore.language = 'en-US'; appStore.saveSettingsToStorage()"
                          class="px-4 py-1.5 rounded-lg text-sm font-black transition-all border border-subtle"
                          :class="appStore.language === 'en-US' ? 'bg-primary text-white border-primary' : 'bg-input text-muted'">{{ appStore.t('settings.lang.en') }}</button>
                </div>
              </div>

              <!-- UI 缩放 -->
              <div class="settings-scale border-t border-subtle">
                <div class="flex justify-between items-center">
                  <div>
                    <span class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.ui_scale') }}</span>
                    <HelpHint label="设置说明">{{ appStore.t('settings.ui_scale.desc') }}</HelpHint>
                  </div>
                  <span class="px-2 py-0.5 rounded-lg bg-primary/10 border border-primary/20 text-primary text-sm font-black font-mono">
                    {{ appStore.settings.uiScale }}%
                  </span>
                </div>
                <input
                  type="range"
                  :aria-label="appStore.t('settings.ui_scale')"
                  v-model.number="appStore.settings.uiScale"
                  min="60" max="200" step="5"
                  @change="validateAndUpdateUIScale(appStore.settings.uiScale)"
                  class="w-full h-1.5 bg-input border border-subtle rounded-full appearance-none cursor-pointer accent-primary"
                />
                <div class="flex justify-between text-xs text-dim font-mono">
                  <span>60%</span><span>100%</span><span>200%</span>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 可访问性设置 -->
        <section v-show="settingsSection==='accessibility'" data-settings-panel="accessibility" class="aero-card">
          <div class="settings-group">
            <div class="settings-group-heading">
              <div class="flex items-center gap-2"><h2 class="text-base font-black text-content uppercase tracking-widest">{{ appStore.t('accessibility.title') }}</h2><HelpHint :label="appStore.t('accessibility.title')">{{ appStore.t('accessibility.subtitle') }}</HelpHint></div>
            </div>

            <div class="settings-group-body">
              <AccessibilitySettings />
            </div>
          </div>
        </section>

        <!-- 核心功能：常规与性能 -->
        <div v-show="settingsSection==='processing' || settingsSection==='behavior'" class="settings-performance-grid">
          <section v-show="settingsSection==='processing'" data-settings-panel="processing" class="aero-card">
            <h2 class="text-sm font-black text-primary uppercase tracking-[0.3em] mb-8">{{ appStore.t('settings.performance') }}</h2>
            <div class="space-y-6">
              <div class="settings-switch-row"><button
                type="button"
                role="switch"
                data-testid="auto-start-switch"
                :aria-checked="appStore.settings.autoStart"
                :aria-busy="autoStartBusy"
                :disabled="!autoStartSupported || autoStartBusy"
                class="w-full flex items-center justify-between gap-4 text-left disabled:cursor-not-allowed disabled:opacity-60"
                @click="toggleAutoStart"
              >
                <div class="min-w-0">
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.performance.auto_start') }}</div>

                </div>
                <div class="settings-toggle-track shrink-0" :class="{ 'is-on': appStore.settings.autoStart }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button><HelpHint :label="appStore.t('settings.performance.auto_start')+'说明'">{{ appStore.t('settings.performance.auto_start.desc') }}</HelpHint></div>

              <!-- 并行线程设置 -->
              <div class="space-y-4 pt-6 border-t border-subtle">
                <div class="flex justify-between items-center">
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.performance.threads') }}</div>
                  <span class="px-2 py-0.5 rounded-lg bg-primary/10 border border-primary/20 text-primary text-sm font-black font-mono">
                    {{ appStore.settings.maxConcurrentTasks }}
                  </span>
                </div>
                <input
                  type="range"
                  :aria-label="appStore.t('settings.performance.threads')"
                  v-model.number="appStore.settings.maxConcurrentTasks"
                  min="1" max="16" step="1"
                  @change="validateAndUpdateThreads(appStore.settings.maxConcurrentTasks)"
                  class="w-full h-1.5 bg-input border border-subtle rounded-full appearance-none cursor-pointer accent-primary"
                />
                <HelpHint label="归档默认设置说明">{{ appStore.t('settings.performance.threads.desc') }}</HelpHint>
              </div>

              <div class="space-y-4 pt-6 border-t border-subtle" data-testid="archive-default-settings">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.archive_defaults') }}</div>
                  <HelpHint label="归档默认设置说明">{{ appStore.t('settings.archive_defaults.desc') }}</HelpHint>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <label class="space-y-1.5 text-xs font-bold text-muted">
                    <span>{{ appStore.t('settings.archive_defaults.format') }}</span>
                    <select
                      :value="appStore.settings.defaultCompressionFormat"
                      class="w-full h-9 rounded-lg border border-subtle bg-input px-3 text-content outline-none focus:border-primary"
                      @change="appStore.updateSettings({ defaultCompressionFormat: ($event.target as HTMLSelectElement).value as 'zip' | '7z' | 'tar.gz' })"
                    >
                      <option value="zip">ZIP</option>
                      <option value="7z">7Z</option>
                      <option value="tar.gz">TAR.GZ</option>
                    </select>
                  </label>
                  <label class="space-y-1.5 text-xs font-bold text-muted">
                    <span>{{ appStore.t('settings.archive_defaults.level') }} · {{ appStore.settings.defaultCompressionLevel }}</span>
                    <input
                      :value="appStore.settings.defaultCompressionLevel"
                      type="range" min="1" max="9" step="1"
                      class="mt-3 w-full h-1.5 rounded-full accent-primary"
                      @change="appStore.updateSettings({ defaultCompressionLevel: Number(($event.target as HTMLInputElement).value) })"
                    />
                  </label>
                </div>
                <div class="settings-switch-row"><button
                  type="button"
                  role="switch"
                  :aria-checked="appStore.settings.defaultExtractToSubfolder"
                  class="w-full flex items-center justify-between gap-4 text-left"
                  @click="appStore.updateSettings({ defaultExtractToSubfolder: !appStore.settings.defaultExtractToSubfolder })"
                >
                  <div class="min-w-0">
                    <div class="text-xs font-bold text-content">{{ appStore.t('settings.archive_defaults.subfolder') }}</div>

                  </div>
                  <div class="settings-toggle-track shrink-0" :class="{ 'is-on': appStore.settings.defaultExtractToSubfolder }">
                    <span class="settings-toggle-knob"></span>
                  </div>
                </button><HelpHint :label="appStore.t('settings.archive_defaults.subfolder')+'说明'">{{ appStore.t('settings.archive_defaults.subfolder.desc') }}</HelpHint></div>
              </div>
            </div>
          </section>

          <section v-show="settingsSection==='processing'" data-settings-panel="processing" class="aero-card">
            <div class="settings-switch-row"><button type="button" role="switch" data-testid="smart-directory-switch"
              :aria-checked="appStore.settings.smartExtractionDirectory"
              class="w-full flex items-center justify-between gap-4 text-left"
              @click="appStore.updateSettings({ smartExtractionDirectory: !appStore.settings.smartExtractionDirectory })">
              <div class="min-w-0">
                <div class="text-sm font-bold text-content">{{ appStore.t('settings.smart_directory') }}</div>

              </div>
              <div class="settings-toggle-track shrink-0" :class="{ 'is-on': appStore.settings.smartExtractionDirectory }"><span class="settings-toggle-knob"></span></div>
            </button><HelpHint :label="appStore.t('settings.smart_directory')+'说明'">{{ appStore.t('settings.smart_directory.desc') }}</HelpHint></div>
          </section>

          <section v-show="settingsSection==='processing'" data-settings-panel="processing" class="aero-card">
            <div class="settings-switch-row"><button type="button" role="switch" data-testid="parallel-zip-switch"
              :aria-checked="appStore.settings.parallelZipExtraction"
              class="w-full flex items-center justify-between gap-4 text-left"
              @click="appStore.updateSettings({ parallelZipExtraction: !appStore.settings.parallelZipExtraction })">
              <div class="min-w-0">
                <div class="text-sm font-bold text-content">{{ appStore.t('settings.parallel_zip') }}</div>

              </div>
              <div class="settings-toggle-track shrink-0" :class="{ 'is-on': appStore.settings.parallelZipExtraction }"><span class="settings-toggle-knob"></span></div>
            </button><HelpHint :label="appStore.t('settings.parallel_zip')+'说明'">{{ appStore.t('settings.parallel_zip.desc') }}</HelpHint></div>
          </section>

          <!-- 暴力破解引擎设置 -->
          <section v-show="settingsSection==='processing'" data-settings-panel="processing" class="aero-card">
            <div class="flex justify-between items-center mb-8">
              <h2 class="text-sm font-black text-muted uppercase tracking-[0.3em]">{{ appStore.t('settings.bruteforce') }}</h2>
              <button type="button" role="switch" :aria-checked="appStore.settings.enableBruteForce" :aria-label="appStore.t('settings.bruteforce')" class="settings-toggle-track cursor-pointer"
                   :class="{ 'is-on': appStore.settings.enableBruteForce }"
                   @click="toggleBruteForce">
                <span class="settings-toggle-knob"></span>
              </button>
            </div>

            <div class="space-y-6" :class="{ 'opacity-80 pointer-events-none': !appStore.settings.enableBruteForce }">
              <div class="space-y-3">
                <div class="flex justify-between items-center">
                  <span class="text-xs font-black text-muted uppercase tracking-widest">{{ appStore.t('settings.bruteforce.wordlists') }}</span>
                  <button @click="addWordlist" class="text-xs font-black text-primary uppercase tracking-widest hover:brightness-110 transition-all flex items-center gap-1">
                    <i class="pi pi-plus text-xs"></i>
                    {{ appStore.t('settings.bruteforce.add') }}
                  </button>
                </div>
                
                <div class="space-y-2 max-h-40 overflow-y-auto custom-scrollbar pr-2">
                  <div v-for="(path, index) in appStore.settings.bruteForceWordlists" :key="path" 
                       class="flex items-center justify-between p-3 rounded-xl bg-input border border-subtle group hover:border-primary/30 transition-all">
                    <span class="text-sm text-content truncate max-w-[200px] font-mono" :title="path">{{ path.split(/[\\/]/).pop() }}</span>
                    <button type="button" @click="removeWordlist(index)" :aria-label="`${appStore.t('common.delete')}: ${path.split(/[\\/]/).pop()}`" class="w-8 h-8 rounded-lg flex items-center justify-center text-muted hover:text-red-500 hover:bg-red-500/10 transition-colors">
                      <i class="pi pi-times text-sm"></i>
                    </button>
                  </div>
                  <div v-if="appStore.settings.bruteForceWordlists.length === 0" class="py-6 text-center border border-dashed border-subtle rounded-xl">
                    <span class="text-xs text-dim uppercase tracking-widest font-bold">{{ appStore.t('settings.bruteforce.empty') }}</span>
                  </div>
                </div>
              </div>
            </div>
          </section>
          <!-- 行为设置 -->
          <section v-show="settingsSection==='behavior'" data-settings-panel="behavior" class="aero-card settings-behavior">
            <h2 class="text-sm font-black text-primary uppercase tracking-[0.3em] mb-6">{{ appStore.t('settings.behavior') }}</h2>
            <div class="settings-behavior-options">
              <div class="settings-switch-row"><button type="button" role="switch" data-testid="close-to-tray-switch" :aria-checked="appStore.settings.closeToTray" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ closeToTray: !appStore.settings.closeToTray })">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.close_to_tray') }}</div>

                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.closeToTray }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button><HelpHint :label="appStore.t('settings.behavior.close_to_tray')+'说明'">{{ appStore.t('settings.behavior.close_to_tray.desc') }}</HelpHint></div>
              <button type="button" role="switch" :aria-checked="appStore.settings.autoDeleteSource" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ autoDeleteSource: !appStore.settings.autoDeleteSource })">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.auto_delete') }}</div>
                  <div class="text-xs text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('settings.behavior.auto_delete.desc') }}</div>
                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.autoDeleteSource }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>
              <div class="settings-switch-row"><button type="button" role="switch" data-testid="preserve-mark-of-web-switch" :aria-checked="appStore.settings.preserveMarkOfWeb" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ preserveMarkOfWeb: !appStore.settings.preserveMarkOfWeb })">
                <div class="min-w-0 pr-4">
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.preserve_motw') }}</div>

                </div>
                <div class="settings-toggle-track shrink-0" :class="{ 'is-on': appStore.settings.preserveMarkOfWeb }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button><HelpHint :label="appStore.t('settings.behavior.preserve_motw')+'说明'">{{ appStore.t('settings.behavior.preserve_motw.desc') }}</HelpHint></div>
              <div class="settings-switch-row"><button type="button" role="switch" :aria-checked="appStore.settings.savePasswords" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ savePasswords: !appStore.settings.savePasswords })">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.save_passwords') }}</div>

                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.savePasswords }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button><HelpHint :label="appStore.t('settings.behavior.save_passwords')+'说明'">{{ appStore.t('settings.behavior.save_passwords.desc') }}</HelpHint></div>
              <div class="settings-switch-row"><button v-if="contextMenuSupported" type="button" role="switch" :aria-checked="appStore.settings.contextMenuEnabled" :disabled="contextMenuBusy" class="w-full flex items-center justify-between group cursor-pointer text-left disabled:opacity-60" @click="toggleContextMenu">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.context_menu') }}</div>

                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.contextMenuEnabled }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button><HelpHint :label="appStore.t('settings.behavior.context_menu')+'说明'">{{ appStore.t('settings.behavior.context_menu.desc') }}</HelpHint></div>
            </div>
          </section>
        </div>

        <!-- 软件更新 -->
        <section v-show="settingsSection==='updates'" data-settings-panel="updates" class="aero-card">
          <div class="settings-group">
            <div class="settings-group-heading">
              <div class="flex items-center gap-2"><h2 class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.update.title') }}</h2><HelpHint :label="appStore.t('settings.update.title')">{{ appStore.t('settings.update.desc') }}</HelpHint></div>
            </div>
            <div class="settings-group-body">
              <div class="flex flex-col gap-4 rounded-2xl border border-subtle bg-input/30 p-5 sm:flex-row sm:items-center sm:justify-between">
                <div>
                  <div class="text-xs font-black uppercase tracking-widest text-muted">{{ appStore.t('settings.update.current') }}</div>
                  <div class="mt-2 flex items-center gap-2 text-lg font-black text-content">
                    <span>v{{ currentVersion }}</span>
                    <span class="rounded-lg border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 text-xs text-emerald-500">Stable</span>
                  </div>
                </div>
                <button type="button" class="h-10 shrink-0 rounded-xl bg-primary px-5 text-xs font-black text-white shadow-lg shadow-primary/20 disabled:cursor-wait disabled:opacity-60" :disabled="updateStore.busy" @click="checkForUpdatesNow">
                  <i class="pi mr-2" :class="updateStore.status === 'checking' ? 'pi-spin pi-spinner' : 'pi-refresh'"></i>{{ appStore.t('settings.update.check') }}
                </button>
              </div>
              <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
                <div class="rounded-xl border border-subtle bg-input/20 p-4">
                  <div class="text-xs font-black uppercase tracking-widest text-muted">{{ appStore.t('settings.update.online') }}</div>
                  <div class="mt-2 text-sm font-black text-content">{{ onlineVersion }}</div>
                </div>
                <div class="rounded-xl border border-subtle bg-input/20 p-4">
                  <div class="text-xs font-black uppercase tracking-widest text-muted">{{ appStore.t('settings.update.result') }}</div>
                  <div class="mt-2 text-sm font-black text-content">{{ updateStatusText }}</div>
                </div>
                <div class="rounded-xl border border-subtle bg-input/20 p-4">
                  <div class="text-xs font-black uppercase tracking-widest text-muted">{{ appStore.t('settings.update.last_attempt') }}</div>
                  <div class="mt-2 text-xs font-bold text-content">{{ formatUpdateTime(updateStore.lastAttemptAt) }}</div>
                </div>
                <div class="rounded-xl border border-subtle bg-input/20 p-4">
                  <div class="text-xs font-black uppercase tracking-widest text-muted">{{ appStore.t('settings.update.last_success') }}</div>
                  <div class="mt-2 text-xs font-bold text-content">{{ formatUpdateTime(updateStore.lastSuccessAt) }}</div>
                </div>
              </div>
              <div v-if="updateStore.status === 'error' && updateStore.errorMessage" class="rounded-xl border border-red-500/20 bg-red-500/5 p-4 text-xs leading-5 text-red-500">
                {{ updateStore.errorMessage }}
              </div>
              <div class="settings-switch-row"><button type="button" role="switch" :aria-checked="appStore.settings.autoCheckUpdates" class="w-full flex items-center justify-between gap-4 text-left" @click="toggleAutoCheckUpdates">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.update.auto') }}</div>

                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.autoCheckUpdates }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button><HelpHint :label="appStore.t('settings.update.auto')+'说明'">{{ appStore.t('settings.update.auto.desc') }}</HelpHint></div>
              <HelpHint label="更新下载说明">{{ appStore.t('settings.update.security') }}</HelpHint>
            </div>
          </div>
        </section>

        <!-- 格式支持统计 -->
        <section v-show="settingsSection==='formats'" data-settings-panel="formats" class="aero-card">
          <div class="settings-group">
            <div class="settings-group-heading">
              <div class="flex items-center gap-2"><h2 class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.formats.title') }}</h2><HelpHint :label="appStore.t('settings.formats.title')">{{ appStore.t('settings.formats.desc') }} {{ appStore.t('settings.formats.count_description') }}</HelpHint></div>
            </div>

            <div class="settings-group-body">
              <div class="rounded-2xl border border-subtle bg-input/30 p-5">
                <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
                  <div class="min-w-0">
                    <div class="flex items-center gap-2 text-sm font-black text-content">
                      <span class="h-2.5 w-2.5 rounded-full" :class="archiveEngine?.fullEngine ? 'bg-emerald-500 shadow-[0_0_12px_rgba(16,185,129,.7)]' : 'bg-amber-500'"></span>
                      {{ appStore.t('settings.engine.title') }}
                    </div>
                    <div class="mt-2 text-xs leading-5 text-muted">
                      {{ archiveEngine?.version ? `7-Zip ${archiveEngine.version}` : appStore.t('settings.engine.unavailable') }}
                      · {{ archiveEngine?.message || appStore.t('settings.engine.detecting') }}
                    </div>
                  </div>
                  <button type="button" class="h-9 shrink-0 rounded-xl border border-subtle bg-input px-4 text-xs font-black text-content transition hover:border-primary disabled:opacity-60" :disabled="diagnosticsLoading" @click="refreshEngineDiagnostics">
                    <i class="pi mr-2" :class="diagnosticsLoading ? 'pi-spin pi-spinner' : 'pi-refresh'"></i>{{ appStore.t('settings.engine.refresh') }}
                  </button>
                </div>
                <div class="mt-3 flex">
                  <div class="rounded-xl border px-3 py-2 text-xs" :class="rarEncoder?.available ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-500' : 'border-amber-500/30 bg-amber-500/10 text-amber-500'">
                    RAR {{ rarEncoder?.available ? appStore.t('settings.engine.ready') : appStore.t('settings.engine.external') }}
                  </div>
                </div>
              </div>

              <!-- 统计卡片组 -->
              <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                <div class="p-6 rounded-2xl bg-gradient-to-br from-emerald-500/10 to-emerald-500/5 border border-emerald-500/20">
                  <div class="text-3xl font-black text-emerald-500 mb-1" data-testid="settings-extraction-count">{{ archiveEngine ? readableExtensionCount : '—' }}</div>
                  <div class="text-xs font-black text-secondary uppercase tracking-widest">{{ appStore.t('settings.formats.decompress') }}</div>
                </div>
                <div class="p-6 rounded-2xl bg-gradient-to-br from-primary/10 to-primary/5 border border-primary/20">
                  <div class="text-3xl font-black text-primary mb-1" data-testid="settings-creation-count">{{ archiveEngine ? creatableFormats.length : '—' }}</div>
                  <div class="text-xs font-black text-secondary uppercase tracking-widest">{{ appStore.t('settings.formats.compress') }}</div>
                </div>
                <div class="p-6 rounded-2xl bg-gradient-to-br from-amber-500/10 to-amber-500/5 border border-amber-500/20">
                  <div class="text-3xl font-black text-amber-500 mb-1" data-testid="settings-password-count">{{ archiveEngine ? passwordExtractionNames.length : '—' }}</div>
                  <div class="text-xs font-black text-secondary uppercase tracking-widest">{{ appStore.t('settings.formats.password') }}</div>
                </div>
              </div>

              <!-- 解压格式详细列表 -->
              <details class="settings-format-details">
                <summary class="text-xs font-black text-primary uppercase tracking-[0.2em]">{{ appStore.t('settings.formats.decompress_list') }}</summary>
                <div class="p-5 rounded-xl bg-input/30 border border-subtle space-y-3">
                  <div class="text-xs text-content leading-relaxed font-mono" data-testid="settings-readable-extensions">{{ readableExtensions.map(extension => extension.toUpperCase()).join(' · ') }}</div>
                  <div v-if="archiveEngine?.formats.some(format => format.name === 'EGG')" data-testid="settings-egg-read">
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">EGG 随附只读引擎<HelpHint label="EGG 支持范围">已验证存储、Deflate、Bzip2、LZMA、AZO、固实及传统 ZipCrypto、AES/LEA 密码读取，支持 UTF-8 名称和已验证的原生 .volN.egg 卷组。固实所选提取仍需读取共享流，缺卷必须补齐。不提供 EGG 创建、文件名/全局加密或损坏修复；具体组合与资源边界见版本说明。</HelpHint></div>
                    <div class="text-xs text-content leading-relaxed">EGG · 密码与固实读取 · 原生分卷 · 检查与转换</div>
                  </div>
                  <div v-if="archiveEngine?.formats.some(format => format.name === 'ALZ')" data-testid="settings-alz-read">
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">ALZ 随附只读引擎<HelpHint label="ALZ 支持范围">已验证存储、Deflate、ALZ 变体 Bzip2、ZipCrypto 和原生 .alz/.aNN 卷组。文件名与密码默认按 CP949，可明确选择原工具代码页；缺卷必须补齐，不提供创建或损坏修复。</HelpHint></div>
                    <div class="text-xs text-content leading-relaxed">ALZ · 密码解压 · 原生分卷 · 检查与转换</div>
                  </div>
                </div>
              </details>

              <!-- 压缩格式详细列表 -->
              <details class="settings-format-details">
                <summary class="text-xs font-black text-primary uppercase tracking-[0.2em]">{{ appStore.t('settings.formats.compress_list') }}</summary>
                  <div class="p-5 rounded-xl bg-input/30 border border-subtle space-y-3">
                  <div v-if="archiveEngine?.formats.some(format => format.name === 'ZPAQ' && format.canCreate)" data-testid="settings-zpaq-creation">
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">ZPAQ 随附引擎</div>
                    <div class="text-xs text-content leading-relaxed">日志式创建与原生密码；支持数字切片分卷，解压需集齐成员。</div>
                  </div>
                  <div class="text-xs text-content leading-relaxed font-mono" data-testid="settings-creation-formats">{{ creatableFormats.map(format => format.displayName).join(' · ') }}</div>
                </div>
              </details>

              <!-- 密码支持说明 -->
              <details class="settings-format-details">
                <summary class="text-xs font-black text-primary uppercase tracking-[0.2em]">{{ appStore.t('settings.formats.password_support') }}</summary>
                <div class="p-5 rounded-xl bg-amber-500/10 border border-amber-500/20 space-y-2">
                  <div class="flex items-start gap-3">
                    <i class="pi pi-lock text-amber-500 text-xs mt-0.5 shrink-0"></i>
                    <div class="text-xs text-secondary leading-relaxed">
                      <span class="font-black">{{ appStore.t('settings.formats.password_compress') }}:</span> {{ passwordCompressionFormats }}
                    </div>
                  </div>
                  <div class="flex items-start gap-3">
                    <i class="pi pi-unlock text-amber-500 text-xs mt-0.5 shrink-0"></i>
                    <div class="text-xs text-secondary leading-relaxed" data-testid="settings-password-extraction-formats">
                      <span class="font-black">{{ appStore.t('settings.formats.password_decompress') }}:</span> {{ passwordExtractionFormats }}
                    </div>
                  </div>
                </div>
              </details>
            </div>
          </div>
        </section>


      </div>
    </div>

    <!-- 重置确认弹窗 -->
    <transition name="pop">
      <div v-if="showResetConfirm" class="fixed inset-0 z-[150] flex items-center justify-center bg-black/60 backdrop-blur-xl p-4" @click.self="showResetConfirm = false">
        <div class="modal-no-glass rounded-3xl p-10 w-full max-w-xs text-center shadow-2xl text-content">
          <h3 class="text-sm font-black mb-2 uppercase tracking-widest">{{ appStore.t('settings.reset.title') }}</h3>
          <p class="text-sm text-muted mb-8">{{ appStore.t('settings.reset.desc') }}</p>
          <div class="flex flex-col gap-2">
            <button @click="appStore.resetSettings(); showResetConfirm = false; appStore.setSuccess(appStore.t('settings.reset.success'))"
                    class="w-full py-3 bg-red-500 text-white rounded-xl text-xs font-black">{{ appStore.t('settings.reset.confirm') }}</button>
            <button @click="showResetConfirm = false"
                    class="w-full py-3 bg-input text-muted rounded-xl text-xs font-bold border border-subtle hover:text-content transition-colors">{{ appStore.t('vault.confirm.cancel') }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped src="./settings-view.css"></style>
