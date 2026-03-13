<script setup lang="ts">
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()

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

const toggleBruteForce = () => appStore.updateSettings({ enableBruteForce: !appStore.settings.enableBruteForce })
const toggleAutoStart = () => appStore.updateSettings({ autoStart: !appStore.settings.autoStart })

const addWordlist = async () => {
  const paths = await tauriCommands.selectWordlists()
  if (paths.length > 0) {
    const current = new Set(appStore.settings.bruteForceWordlists)
    paths.forEach(p => current.add(p))
    appStore.updateSettings({ bruteForceWordlists: Array.from(current) })
    appStore.successMessage = appStore.t('common.success')
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
      <p class="text-muted text-[10px] font-bold uppercase tracking-[0.3em] ml-1">{{ appStore.t('settings.subtitle') }}</p>
    </header>

    <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-20">
      <div class="max-w-5xl space-y-6">
        <!-- 风格大一统：外观个性化 (左右分栏布局) -->
        <section class="aero-card p-10 overflow-hidden">
          <div class="grid grid-cols-1 lg:grid-cols-12 gap-12">
            <div class="lg:col-span-4 space-y-2">
              <h2 class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('settings.appearance') }}</h2>
              <p class="text-[10px] text-muted leading-relaxed uppercase tracking-tighter">{{ appStore.t('settings.appearance.desc') }}</p>
            </div>
            
            <div class="lg:col-span-8 space-y-10">
              <!-- 模式切换 (进化版) -->
              <div class="space-y-4">
                <label class="text-[9px] font-black text-muted uppercase tracking-[0.2em] block ml-1">{{ appStore.t('settings.theme') }}</label>
                <div class="grid grid-cols-2 sm:grid-cols-3 p-1 rounded-2xl bg-input border border-subtle gap-1">
                  <button 
                    v-for="m in themeModes" :key="m.value"
                    @click="appStore.theme = m.value as any; appStore.saveSettingsToStorage()"
                    class="py-3 rounded-xl text-[9px] font-black uppercase transition-all flex items-center justify-center gap-2"
                    :class="appStore.theme === m.value ? 'bg-primary text-white shadow-lg' : 'text-muted hover:bg-white/5'"
                  >
                    <i :class="m.icon"></i>
                    {{ appStore.t(m.label) }}
                  </button>
                </div>
              </div>

              <!-- 强调色选择 -->
              <div class="space-y-4">
                <label class="text-[9px] font-black text-muted uppercase tracking-[0.2em] block ml-1">{{ appStore.t('settings.accent') }}</label>
                <div class="flex flex-wrap gap-3 p-1">
                  <button 
                    v-for="(hex, name) in themeColors" :key="name"
                    @click="appStore.accentColor = hex; appStore.saveSettingsToStorage()"
                    class="w-7 h-7 rounded-full border-4 transition-all hover:scale-110 shadow-sm"
                    :style="{ backgroundColor: hex, borderColor: appStore.accentColor === hex ? 'var(--text-base)' : 'transparent' }"
                    :title="name"
                  ></button>
                </div>
              </div>

              <!-- 语言选择 -->
              <div class="pt-6 border-t border-subtle flex items-center justify-between">
                <span class="text-[10px] font-black text-content uppercase tracking-widest">{{ appStore.t('settings.language') }}</span>
                <div class="flex gap-2">
                  <button @click="appStore.language = 'zh-CN'; appStore.saveSettingsToStorage()" 
                          class="px-4 py-1.5 rounded-lg text-[10px] font-black transition-all border border-subtle"
                          :class="appStore.language === 'zh-CN' ? 'bg-primary text-white border-primary' : 'bg-input text-muted'">简体中文</button>
                  <button @click="appStore.language = 'en-US'; appStore.saveSettingsToStorage()"
                          class="px-4 py-1.5 rounded-lg text-[10px] font-black transition-all border border-subtle"
                          :class="appStore.language === 'en-US' ? 'bg-primary text-white border-primary' : 'bg-input text-muted'">English</button>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- 核心功能：常规与性能 -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <section class="aero-card p-8">
            <h2 class="text-[10px] font-black text-primary uppercase tracking-[0.3em] mb-8">{{ appStore.t('settings.performance') }}</h2>
            <div class="space-y-6">
              <!-- 自启动开关 -->
              <div class="flex items-center justify-between group cursor-pointer" @click="toggleAutoStart">
                <div>
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.performance.auto_start') }}</div>
                  <div class="text-[9px] text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('settings.performance.auto_start.desc') }}</div>
                </div>
                <div class="w-10 h-5 rounded-full border border-subtle p-0.5 transition-all" :class="appStore.settings.autoStart ? 'bg-primary/40 border-primary' : 'bg-input'">
                  <div class="w-3.5 h-3.5 rounded-full bg-white shadow-sm transition-all" :class="appStore.settings.autoStart ? 'translate-x-5' : ''"></div>
                </div>
              </div>

              <!-- 并行线程设置 -->
              <div class="space-y-4 pt-6 border-t border-subtle">
                <div class="flex justify-between items-center">
                  <div class="text-xs font-bold text-content">{{ appStore.t('settings.performance.threads') }}</div>
                  <span class="px-2 py-0.5 rounded-lg bg-primary/10 border border-primary/20 text-primary text-[10px] font-black font-mono">
                    {{ appStore.settings.maxConcurrentTasks }}
                  </span>
                </div>
                <input
                  type="range"
                  v-model.number="appStore.settings.maxConcurrentTasks"
                  min="1" max="16" step="1"
                  @input="appStore.saveSettingsToStorage()"
                  class="w-full h-1.5 bg-input border border-subtle rounded-full appearance-none cursor-pointer accent-primary"
                />
                <div class="text-[8px] text-muted uppercase tracking-tighter">{{ appStore.t('settings.performance.threads.desc') }}</div>
              </div>
            </div>
          </section>

          <!-- 暴力破解引擎设置 -->
          <section class="aero-card p-8">
            <div class="flex justify-between items-center mb-8">
              <h2 class="text-[10px] font-black text-muted uppercase tracking-[0.3em]">{{ appStore.t('settings.bruteforce') }}</h2>
              <div class="w-10 h-5 rounded-full border border-subtle p-0.5 transition-all cursor-pointer" 
                   :class="appStore.settings.enableBruteForce ? 'bg-primary/40 border-primary' : 'bg-input'"
                   @click="toggleBruteForce">
                <div class="w-3.5 h-3.5 rounded-full bg-white shadow-sm transition-all" :class="appStore.settings.enableBruteForce ? 'translate-x-5' : ''"></div>
              </div>
            </div>

            <div class="space-y-6" :class="{ 'opacity-40 pointer-events-none': !appStore.settings.enableBruteForce }">
              <div class="space-y-3">
                <div class="flex justify-between items-center">
                  <span class="text-[9px] font-black text-muted uppercase tracking-widest">{{ appStore.t('settings.bruteforce.wordlists') }}</span>
                  <button @click="addWordlist" class="text-[9px] font-black text-primary uppercase tracking-widest hover:brightness-110 transition-all flex items-center gap-1">
                    <i class="pi pi-plus text-[8px]"></i>
                    {{ appStore.t('settings.bruteforce.add') }}
                  </button>
                </div>
                
                <div class="space-y-2 max-h-40 overflow-y-auto custom-scrollbar pr-2">
                  <div v-for="(path, index) in appStore.settings.bruteForceWordlists" :key="path" 
                       class="flex items-center justify-between p-3 rounded-xl bg-input border border-subtle group hover:border-primary/30 transition-all">
                    <span class="text-[10px] text-content truncate max-w-[200px] font-mono" :title="path">{{ path.split(/[\\/]/).pop() }}</span>
                    <i @click="removeWordlist(index)" class="pi pi-times text-[10px] text-muted hover:text-red-500 cursor-pointer transition-colors"></i>
                  </div>
                  <div v-if="appStore.settings.bruteForceWordlists.length === 0" class="py-6 text-center border border-dashed border-subtle rounded-xl">
                    <span class="text-[9px] text-dim uppercase tracking-widest font-bold">{{ appStore.t('settings.bruteforce.empty') }}</span>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  background: radial-gradient(circle at 100% 100%, color-mix(in srgb, var(--dynamic-accent) 3%, transparent) 0%, transparent 50%);
}
</style>
