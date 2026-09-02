<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getVersion } from '@tauri-apps/api/app'
import { appWindow } from '@tauri-apps/api/window'
import WindowTitleBar from '@/components/layouts/WindowTitleBar.vue'
import GlobalProgressBar from '@/components/ui/GlobalProgressBar.vue'
import { useAppStore } from '@/stores/app'
import brandIcon from '@/assets/long-jieya-icon.png'

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const isFocused = ref(true)
const appVersion = ref('')
let unlistenFocus: any = null
let isUnmounted = false

onMounted(async () => {
  try {
    appVersion.value = await getVersion()
  } catch {
    // 浏览器预览没有原生应用信息；不展示容易误导的硬编码版本。
  }
  try {
    const unlisten = await appWindow.onFocusChanged(({ payload: focused }) => {
      isFocused.value = focused
    })
    if (isUnmounted) unlisten()
    else unlistenFocus = unlisten
  } catch (error) {
    console.warn('Window focus listener is unavailable:', error)
  }
})

onUnmounted(() => {
  isUnmounted = true
  if (unlistenFocus) unlistenFocus()
})

const navItems = [
  { name: 'Decompress', icon: 'pi pi-folder-open', label: 'nav.decompress', shortcut: 'Ctrl+O' },
  { name: 'Compress', icon: 'pi pi-box', label: 'nav.compress', shortcut: 'Ctrl+N' },
  { name: 'SpecialCompression', icon: 'pi pi-sparkles', label: 'nav.special_compression', shortcut: 'Ctrl+Shift+S' },
  { name: 'ArchiveBrowser', icon: 'pi pi-folder', label: 'nav.browser', shortcut: 'Ctrl+B' },
  { name: 'Vault', icon: 'pi pi-shield', label: 'nav.vault', shortcut: 'Ctrl+Shift+V' },
  { name: 'FileIntegrity', icon: 'pi pi-verified', label: 'nav.integrity', shortcut: 'Ctrl+I' },
  { name: 'History', icon: 'pi pi-history', label: 'nav.history', shortcut: 'Ctrl+H' },
  { name: 'Settings', icon: 'pi pi-cog', label: 'nav.settings', shortcut: 'Ctrl+,' }
]

const navigateTo = (name: string) => {
  router.push({ name })
}
</script>

<template>
  <!-- 主容器：通过 p-[1px] 留出系统 Resize 缓冲带 -->
  <div 
    class="main-container flex flex-col h-screen w-screen bg-transparent p-[1px] overflow-hidden"
    style="box-sizing: border-box;"
  >
    <div class="flex-1 flex flex-col overflow-hidden bg-base text-content rounded-xl relative border transition-all duration-300"
         :class="[isFocused ? 'border-primary/20 shadow-[0_8px_32px_rgba(0,0,0,0.2)]' : 'border-subtle shadow-sm']">

      <WindowTitleBar class="shrink-0" />

      <div class="main-layout flex flex-1 overflow-hidden relative">
        <!-- 侧边栏 - 华丽扁平版 -->
        <aside class="app-sidebar h-full flex flex-col border-r border-subtle/60 bg-card/75 backdrop-blur-2xl z-50 shrink-0 relative">
          <div class="sidebar-brand flex items-center gap-3 px-4 h-20 border-b border-subtle/50 shrink-0">
            <div class="w-10 h-10 flex items-center justify-center shrink-0">
              <img :src="brandIcon" alt="" class="w-10 h-10 object-contain" aria-hidden="true">
            </div>
            <div class="sidebar-copy min-w-0">
              <div class="text-sm font-black text-content tracking-tight">Long解压</div>
              <div class="text-xs text-muted tracking-wider mt-0.5">Archive Studio</div>
            </div>
          </div>
          <nav class="flex-1 flex flex-col gap-2 w-full p-3 overflow-y-auto overflow-x-hidden custom-scrollbar">
            <button v-for="item in navItems" :key="item.name"
                  type="button"
                  @click="navigateTo(item.name)"
                  :data-testid="`nav-${item.name}`"
                  :aria-label="item.shortcut ? `${appStore.t(item.label)} (${item.shortcut})` : appStore.t(item.label)"
                 :aria-current="route.name === item.name ? 'page' : undefined"
                 class="nav-entry group relative w-full h-12 flex items-center gap-3 px-3 rounded-xl cursor-pointer transition-all duration-200 text-left"
                 :class="route.name === item.name ? 'bg-primary/20 shadow-sm text-primary' : 'text-muted hover:bg-primary/8 hover:text-content'">

              <div class="absolute left-0 w-1 h-7 rounded-full bg-gradient-to-b from-primary to-primary/50 transition-all duration-300 shadow-[0_0_8px_var(--dynamic-accent)]"
                   :class="route.name === item.name ? 'scale-y-100 opacity-100' : 'scale-y-0 opacity-0'"></div>

              <div class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 transition-colors"
                   :class="route.name === item.name ? 'bg-primary/15' : 'bg-input/70 group-hover:bg-primary/10'">
                <i :class="[item.icon, 'text-base transition-all duration-200', route.name === item.name ? 'text-primary' : 'text-muted group-hover:text-content']"></i>
              </div>
              <div class="sidebar-copy flex-1 min-w-0">
                <div class="text-xs font-extrabold truncate">{{ appStore.t(item.label) }}</div>
                <div v-if="item.shortcut" class="text-xs text-dim font-mono mt-0.5">{{ item.shortcut }}</div>
              </div>
            </button>
          </nav>
          <div class="sidebar-task-area px-3 pb-2 shrink-0">
            <GlobalProgressBar />
          </div>
          <div class="sidebar-copy mx-3 mb-3 p-3 rounded-xl bg-input/45 border border-subtle/60">
            <div class="flex items-center gap-2 text-xs font-bold text-muted">
              <i class="pi pi-sparkles text-primary"></i>
              <span>本地处理 · 隐私优先</span>
            </div>
          </div>
          <div
            v-if="appVersion"
            class="sidebar-version-row mx-3 mb-3 px-2.5 py-2 rounded-xl border border-primary/15 bg-primary/5 flex items-center justify-between gap-2"
            :title="`Long解压 v${appVersion}`"
          >
            <span class="sidebar-version-label text-[10px] font-black tracking-[0.12em] text-dim uppercase">Long解压</span>
            <span data-testid="sidebar-version-badge" class="sidebar-version-badge rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-[10px] font-black text-primary tabular-nums whitespace-nowrap">v{{ appVersion }}</span>
          </div>
        </aside>

        <!-- 主内容区 -->
        <main class="app-main flex-1 relative h-full overflow-hidden min-w-0 z-10 bg-base">
          <router-view v-slot="{ Component }">
            <transition name="aero-page" mode="out-in">
              <div :key="route.path" class="h-full w-full overflow-hidden">
                <component :is="Component" />
              </div>
            </transition>
          </router-view>

        </main>
      </div>
    </div>
  </div>
</template>

<style>
html, body, #app {
  background-color: transparent !important;
  margin: 0;
  padding: 0;
  overflow: hidden;
  height: 100vh;
  width: 100vw;
}

.aero-page-enter-active {
  transition: opacity 0.18s cubic-bezier(0.4, 0, 0.2, 1), transform 0.18s cubic-bezier(0.4, 0, 0.2, 1);
}

.aero-page-leave-active {
  transition: opacity 0.12s cubic-bezier(0.4, 0, 1, 1), transform 0.12s cubic-bezier(0.4, 0, 1, 1);
}

.aero-page-enter-from {
  opacity: 0;
  transform: translateY(6px) scale(0.99);
}

.aero-page-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(1.005);
}

.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
  height: 6px;
  background: transparent;
}

.custom-scrollbar {
  scrollbar-width: thin;
  scrollbar-color: var(--dynamic-accent) transparent;
}

.custom-scrollbar::-webkit-scrollbar-track,
.custom-scrollbar::-webkit-scrollbar-corner {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-button {
  display: none;
  width: 0;
  height: 0;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--dynamic-accent);
  border-radius: 10px;
  transition: background 0.3s ease;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: var(--dynamic-accent-alt);
}
</style>

<style scoped>
.app-sidebar {
  width: 13.5rem;
  transition: width 0.25s ease;
}

.app-main::before {
  content: '';
  position: absolute;
  inset: 0;
  pointer-events: none;
  background:
    radial-gradient(circle at 88% 8%, color-mix(in srgb, var(--dynamic-accent) 9%, transparent), transparent 34%),
    radial-gradient(circle at 5% 92%, color-mix(in srgb, var(--dynamic-accent) 5%, transparent), transparent 28%);
  z-index: -1;
}

@media (max-width: 840px) {
  .app-sidebar { width: 4.75rem; }
  .sidebar-copy { display: none; }
  .sidebar-brand { justify-content: center; padding-inline: 0.75rem; }
  .nav-entry { justify-content: center; padding-inline: 0.5rem; }
  .sidebar-task-area { padding-inline: 1rem; }
  .sidebar-version-row { margin-inline: 0.5rem; padding-inline: 0.25rem; justify-content: center; }
  .sidebar-version-label { display: none; }
  .sidebar-version-badge { padding-inline: 0.35rem; font-size: 0.55rem; letter-spacing: -0.02em; }
}
</style>
