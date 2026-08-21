<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useArchiveEngine } from '@/composables/useArchiveEngine'
import { FORMAT_CAPABILITIES } from '@/utils/compressionFormat'
import AccessibilitySettings from '@/components/settings/AccessibilitySettings.vue'
import { useUpdateStore } from '@/stores/update'

const appStore = useAppStore()
const updateStore = useUpdateStore()
const tauriCommands = useTauriCommands()
const { capabilities: archiveEngine, loading: archiveEngineLoading, refresh: refreshArchiveEngine } = useArchiveEngine()
const showResetConfirm = ref(false)
const rarEncoder = ref<{ available: boolean; message: string } | null>(null)
const currentVersion = ref('—')
const diagnosticsLoading = computed(() => archiveEngineLoading.value)
const readableExtensionCount = computed(() => new Set(archiveEngine.value?.formats.flatMap(format => format.extensions) || []).size)
const creatableFormatCount = computed(() => archiveEngine.value?.formats.filter(format => format.canCreate).length || 0)
const passwordCompressionFormats = computed(() => FORMAT_CAPABILITIES
  .filter(format => format.supportsPasswordCompress)
  .map(format => format.displayName)
  .join(' · '))
const passwordExtractionFormats = computed(() => FORMAT_CAPABILITIES
  .filter(format => format.supportsPasswordExtract)
  .map(format => format.displayName)
  .join(' · '))

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

const themeModes = [
  { value: 'light', icon: 'pi pi-sun', label: 'settings.theme.light' },
  { value: 'dark', icon: 'pi pi-moon', label: 'settings.theme.dark' },
  { value: 'cyberpunk', icon: 'pi pi-bolt', label: 'settings.theme.cyberpunk' },
  { value: 'twilight', icon: 'pi pi-star', label: 'settings.theme.twilight' },
  { value: 'sepia', icon: 'pi pi-book', label: 'settings.theme.sepia' },
  { value: 'auto', icon: 'pi pi-desktop', label: 'settings.theme.auto' }
]

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
  <div class="settings-view p-responsive p-8 h-screen flex flex-col gap-8 transition-colors duration-700 overflow-hidden">
    <header class="shrink-0">
      <h1 class="text-4xl font-black text-content tracking-tighter mb-2">{{ appStore.t('settings.title') }}</h1>
      <p class="text-muted text-sm font-bold uppercase tracking-[0.3em] ml-1">{{ appStore.t('settings.subtitle') }}</p>
    </header>

    <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-20">
      <div class="max-w-5xl space-y-6">
        <!-- 风格大一统：外观个性化 (左右分栏布局) -->
        <section class="aero-card p-10 overflow-hidden">
          <div class="grid grid-cols-1 lg:grid-cols-12 gap-12">
            <div class="lg:col-span-4 space-y-2">
              <h2 class="text-base font-black text-content uppercase tracking-widest">{{ appStore.t('settings.appearance') }}</h2>
              <p class="text-sm text-muted leading-relaxed uppercase tracking-tighter">{{ appStore.t('settings.appearance.desc') }}</p>
            </div>
            
            <div class="lg:col-span-8 space-y-10">
              <!-- 模式切换 (进化版) -->
              <div class="space-y-4">
                <label class="text-xs font-black text-muted uppercase tracking-[0.2em] block ml-1">{{ appStore.t('settings.theme') }}</label>
                <div class="grid grid-cols-2 sm:grid-cols-3 p-1 rounded-2xl bg-input border border-subtle gap-1">
                  <button
                    v-for="m in themeModes" :key="m.value"
                    @click="appStore.theme = m.value as any; appStore.saveSettingsToStorage()"
                    class="py-3 rounded-xl text-xs font-black uppercase transition-all flex items-center justify-center gap-2"
                    :class="appStore.theme === m.value ? 'bg-primary text-white shadow-lg' : 'text-muted hover:bg-white/5'"
                  >
                    <i :class="m.icon"></i>
                    {{ appStore.t(m.label) }}
                  </button>
                </div>
              </div>

              <!-- 强调色选择 -->
              <div class="space-y-4">
                <div class="flex items-center justify-between gap-4 ml-1">
                  <label class="text-xs font-black text-muted uppercase tracking-[0.2em]">{{ appStore.t('settings.accent') }}</label>
                  <span class="rounded-lg border border-primary/30 bg-primary/10 px-2.5 py-1 text-xs font-black uppercase tracking-wider text-primary">{{ selectedAccentName }}</span>
                </div>
                <div class="flex flex-wrap gap-3 rounded-2xl border border-subtle bg-input/35 p-3">
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
              <div class="pt-6 border-t border-subtle flex items-center justify-between">
                <span class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.language') }}</span>
                <div class="flex gap-2">
                  <button @click="appStore.language = 'zh-CN'; appStore.saveSettingsToStorage()" 
                          class="px-4 py-1.5 rounded-lg text-sm font-black transition-all border border-subtle"
                          :class="appStore.language === 'zh-CN' ? 'bg-primary text-white border-primary' : 'bg-input text-muted'">{{ appStore.t('settings.lang.zh') }}</button>
                  <button @click="appStore.language = 'en-US'; appStore.saveSettingsToStorage()"
                          class="px-4 py-1.5 rounded-lg text-sm font-black transition-all border border-subtle"
                          :class="appStore.language === 'en-US' ? 'bg-primary text-white border-primary' : 'bg-input text-muted'">{{ appStore.t('settings.lang.en') }}</button>
                </div>
              </div>

              <!-- UI 缩放 -->
              <div class="pt-6 border-t border-subtle space-y-3">
                <div class="flex justify-between items-center">
                  <div>
                    <span class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.ui_scale') }}</span>
                    <div class="text-xs text-muted mt-0.5">{{ appStore.t('settings.ui_scale.desc') }}</div>
                  </div>
                  <span class="px-2 py-0.5 rounded-lg bg-primary/10 border border-primary/20 text-primary text-sm font-black font-mono">
                    {{ appStore.settings.uiScale }}%
                  </span>
                </div>
                <input
                  type="range"
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
        <section class="aero-card p-10 overflow-hidden">
          <div class="grid grid-cols-1 lg:grid-cols-12 gap-12">
            <div class="lg:col-span-4 space-y-2">
              <h2 class="text-base font-black text-content uppercase tracking-widest">{{ appStore.t('accessibility.title') }}</h2>
              <p class="text-sm text-muted leading-relaxed uppercase tracking-tighter">{{ appStore.t('accessibility.subtitle') }}</p>
            </div>

            <div class="lg:col-span-8">
              <AccessibilitySettings />
            </div>
          </div>
        </section>

        <!-- 核心功能：常规与性能 -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <section class="aero-card p-8">
            <h2 class="text-sm font-black text-primary uppercase tracking-[0.3em] mb-8">{{ appStore.t('settings.performance') }}</h2>
            <div class="space-y-6">
              <button
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
                  <div class="text-xs text-muted mt-1 leading-5">{{ appStore.t('settings.performance.auto_start.desc') }}</div>
                </div>
                <div class="settings-toggle-track shrink-0" :class="{ 'is-on': appStore.settings.autoStart }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>

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
                  v-model.number="appStore.settings.maxConcurrentTasks"
                  min="1" max="16" step="1"
                  @change="validateAndUpdateThreads(appStore.settings.maxConcurrentTasks)"
                  class="w-full h-1.5 bg-input border border-subtle rounded-full appearance-none cursor-pointer accent-primary"
                />
                <div class="text-xs text-muted uppercase tracking-tighter">{{ appStore.t('settings.performance.threads.desc') }}</div>
              </div>
            </div>
          </section>

          <!-- 暴力破解引擎设置 -->
          <section class="aero-card p-8">
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
          <section class="aero-card p-8">
            <h2 class="text-sm font-black text-primary uppercase tracking-[0.3em] mb-6">{{ appStore.t('settings.behavior') }}</h2>
            <div class="space-y-5">
              <button type="button" role="switch" :aria-checked="appStore.settings.closeToTray" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ closeToTray: !appStore.settings.closeToTray })">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.close_to_tray') }}</div>
                  <div class="text-xs text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('settings.behavior.close_to_tray.desc') }}</div>
                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.closeToTray }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>
              <button type="button" role="switch" :aria-checked="appStore.settings.autoDeleteSource" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ autoDeleteSource: !appStore.settings.autoDeleteSource })">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.auto_delete') }}</div>
                  <div class="text-xs text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('settings.behavior.auto_delete.desc') }}</div>
                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.autoDeleteSource }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>
              <button type="button" role="switch" data-testid="preserve-mark-of-web-switch" :aria-checked="appStore.settings.preserveMarkOfWeb" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ preserveMarkOfWeb: !appStore.settings.preserveMarkOfWeb })">
                <div class="min-w-0 pr-4">
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.preserve_motw') }}</div>
                  <div class="text-xs text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('settings.behavior.preserve_motw.desc') }}</div>
                </div>
                <div class="settings-toggle-track shrink-0" :class="{ 'is-on': appStore.settings.preserveMarkOfWeb }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>
              <button type="button" role="switch" :aria-checked="appStore.settings.savePasswords" class="w-full flex items-center justify-between group cursor-pointer text-left" @click="appStore.updateSettings({ savePasswords: !appStore.settings.savePasswords })">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.save_passwords') }}</div>
                  <div class="text-xs text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('settings.behavior.save_passwords.desc') }}</div>
                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.savePasswords }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>
              <button v-if="contextMenuSupported" type="button" role="switch" :aria-checked="appStore.settings.contextMenuEnabled" :disabled="contextMenuBusy" class="w-full flex items-center justify-between group cursor-pointer text-left disabled:opacity-60" @click="toggleContextMenu">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.behavior.context_menu') }}</div>
                  <div class="text-xs text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('settings.behavior.context_menu.desc') }}</div>
                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.contextMenuEnabled }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>
            </div>
          </section>
        </div>

        <!-- 软件更新 -->
        <section class="aero-card p-10 overflow-hidden">
          <div class="grid grid-cols-1 lg:grid-cols-12 gap-12">
            <div class="lg:col-span-4 space-y-2">
              <h2 class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.update.title') }}</h2>
              <p class="text-sm text-muted leading-relaxed uppercase tracking-tighter">{{ appStore.t('settings.update.desc') }}</p>
            </div>
            <div class="lg:col-span-8 space-y-5">
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
              <button type="button" role="switch" :aria-checked="appStore.settings.autoCheckUpdates" class="w-full flex items-center justify-between gap-4 text-left" @click="toggleAutoCheckUpdates">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.update.auto') }}</div>
                  <div class="mt-1 text-xs text-muted uppercase tracking-tighter">{{ appStore.t('settings.update.auto.desc') }}</div>
                </div>
                <div class="settings-toggle-track" :class="{ 'is-on': appStore.settings.autoCheckUpdates }">
                  <span class="settings-toggle-knob"></span>
                </div>
              </button>
              <div class="flex items-start gap-3 rounded-xl border border-primary/15 bg-primary/5 p-4 text-xs leading-5 text-muted">
                <i class="pi pi-shield mt-0.5 text-primary"></i>
                <span>{{ appStore.t('settings.update.security') }}</span>
              </div>
            </div>
          </div>
        </section>

        <!-- 格式支持统计 -->
        <section class="aero-card p-10 overflow-hidden">
          <div class="grid grid-cols-1 lg:grid-cols-12 gap-12">
            <div class="lg:col-span-4 space-y-2">
              <h2 class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.formats.title') }}</h2>
              <p class="text-sm text-muted leading-relaxed uppercase tracking-tighter">{{ appStore.t('settings.formats.desc') }}</p>
            </div>

            <div class="lg:col-span-8 space-y-8">
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
                <div class="mt-4 grid grid-cols-1 gap-2 sm:grid-cols-3">
                  <div class="rounded-xl border border-subtle px-3 py-2 text-xs text-muted"><span class="font-black text-content">{{ readableExtensionCount }}</span> {{ appStore.t('settings.engine.readable') }}</div>
                  <div class="rounded-xl border border-subtle px-3 py-2 text-xs text-muted"><span class="font-black text-content">{{ creatableFormatCount }}</span> {{ appStore.t('settings.engine.creatable') }}</div>
                  <div class="rounded-xl border px-3 py-2 text-xs" :class="rarEncoder?.available ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-500' : 'border-amber-500/30 bg-amber-500/10 text-amber-500'">
                    RAR {{ rarEncoder?.available ? appStore.t('settings.engine.ready') : appStore.t('settings.engine.external') }}
                  </div>
                </div>
              </div>

              <!-- 统计卡片组 -->
              <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                <div class="p-6 rounded-2xl bg-gradient-to-br from-emerald-500/10 to-emerald-500/5 border border-emerald-500/20">
                  <div class="text-3xl font-black text-emerald-500 mb-1">37+</div>
                  <div class="text-xs font-black text-emerald-600/80 uppercase tracking-widest">{{ appStore.t('settings.formats.decompress') }}</div>
                </div>
                <div class="p-6 rounded-2xl bg-gradient-to-br from-primary/10 to-primary/5 border border-primary/20">
                  <div class="text-3xl font-black text-primary mb-1">16</div>
                  <div class="text-xs font-black text-primary/80 uppercase tracking-widest">{{ appStore.t('settings.formats.compress') }}</div>
                </div>
                <div class="p-6 rounded-2xl bg-gradient-to-br from-amber-500/10 to-amber-500/5 border border-amber-500/20">
                  <div class="text-3xl font-black text-amber-500 mb-1">3</div>
                  <div class="text-xs font-black text-amber-600/80 uppercase tracking-widest">{{ appStore.t('settings.formats.password') }}</div>
                </div>
              </div>

              <!-- 解压格式详细列表 -->
              <div class="space-y-3">
                <h3 class="text-xs font-black text-primary uppercase tracking-[0.2em] ml-1">{{ appStore.t('settings.formats.decompress_list') }}</h3>
                <div class="p-5 rounded-xl bg-input/30 border border-subtle space-y-3">
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.archives') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      ZIP · ZIPX · 7Z · RAR · TAR · TAR.GZ · TGZ · TAR.BZ2 · TBZ · TAR.XZ · TXZ · TAR.ZST · TZST · GZ · GZIP · BZ2 · BZIP2 · XZ · ZST · ZSTD · LZMA · OVA
                    </div>
                  </div>
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.containers') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      JAR · XPI · ODT · ODS · DOCX · XLSX · PPTX · EPUB · IPA · APK · APPX
                    </div>
                  </div>
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.disk_images') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      ISO · IMG · DMG · WIM · VHD · VHDX · QCOW · QCOW2 · VDI · VMDK
                    </div>
                  </div>
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.installers') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      CAB · DEB · UDEB · RPM · MSI · MSP · MSM · NSIS · PPKG
                    </div>
                  </div>
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.legacy') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      AR · A · LZH · LHA · ARJ · CHM · SQUASHFS · SFS · XAR · CPIO · UDF · FAT · NTFS · HFS · APFS · EXT2 · EXT3 · EXT4 · GPT · MBR · CRAMFS · ALZ · ARC
                    </div>
                  </div>
                </div>
              </div>

              <!-- 压缩格式详细列表 -->
              <div class="space-y-3">
                <h3 class="text-xs font-black text-primary uppercase tracking-[0.2em] ml-1">{{ appStore.t('settings.formats.compress_list') }}</h3>
                <div class="p-5 rounded-xl bg-input/30 border border-subtle space-y-3">
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.native_rust') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      ZIP · 7Z · TAR · TAR.GZ · TAR.BZ2 · TAR.XZ · GZ · BZ2 · XZ
                    </div>
                  </div>
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.via_7z') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      TAR.ZST · ZST · ZSTD · LZMA · WIM
                    </div>
                  </div>
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.via_winrar') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      RAR ({{ appStore.t('settings.formats.requires_winrar') }})
                    </div>
                  </div>
                  <div>
                    <div class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('settings.formats.category.split') }}</div>
                    <div class="text-xs text-content leading-relaxed font-mono">
                      ZIP ({{ appStore.t('settings.formats.multipart') }})
                    </div>
                  </div>
                </div>
              </div>

              <!-- 密码支持说明 -->
              <div class="space-y-3">
                <h3 class="text-xs font-black text-amber-600 uppercase tracking-[0.2em] ml-1">{{ appStore.t('settings.formats.password_support') }}</h3>
                <div class="p-5 rounded-xl bg-amber-500/10 border border-amber-500/20 space-y-2">
                  <div class="flex items-start gap-3">
                    <i class="pi pi-lock text-amber-500 text-xs mt-0.5 shrink-0"></i>
                    <div class="text-xs text-amber-700 leading-relaxed">
                      <span class="font-black">{{ appStore.t('settings.formats.password_compress') }}:</span> {{ passwordCompressionFormats }}
                    </div>
                  </div>
                  <div class="flex items-start gap-3">
                    <i class="pi pi-unlock text-amber-500 text-xs mt-0.5 shrink-0"></i>
                    <div class="text-xs text-amber-700 leading-relaxed">
                      <span class="font-black">{{ appStore.t('settings.formats.password_decompress') }}:</span> {{ passwordExtractionFormats }}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 重置按钮 -->
        <div class="pt-8 border-t border-subtle flex justify-end">
          <button @click="showResetConfirm = true"
                  class="px-4 py-2 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 hover:bg-red-500 hover:text-white transition-all text-sm font-black uppercase tracking-widest">
            {{ appStore.t('settings.reset.button') }}
          </button>
        </div>
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

<style scoped>
.settings-view {
  background: radial-gradient(circle at 100% 100%, color-mix(in srgb, var(--dynamic-accent) 3%, transparent) 0%, transparent 50%);
}
</style>
