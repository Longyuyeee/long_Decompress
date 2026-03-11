<script setup lang="ts">
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()

const toggleBruteForce = () => {
  appStore.updateSettings({ enableBruteForce: !appStore.settings.enableBruteForce })
}

const toggleAutoDelete = () => {
  appStore.updateSettings({ autoDeleteSource: !appStore.settings.autoDeleteSource })
}
</script>

<template>
  <div class="settings-view p-8 min-h-screen">
    <header class="mb-10 flex justify-between items-end">
      <div>
        <h1 class="text-4xl font-black text-white tracking-tighter mb-2">全局设置</h1>
        <p class="text-white/40 text-sm font-medium tracking-wide uppercase">Global Configuration</p>
      </div>
    </header>

    <div class="max-w-3xl space-y-8">
      <!-- 常规设置 -->
      <section class="p-8 rounded-3xl border border-white/10 backdrop-blur-xl bg-white/5 shadow-2xl">
        <h2 class="text-xs font-black text-white/50 uppercase tracking-widest mb-6">常规设置</h2>
        <div class="space-y-6">
          <div class="flex items-center justify-between group cursor-pointer" @click="toggleAutoDelete">
            <div>
              <div class="text-white font-medium mb-1 group-hover:text-blue-400 transition-colors">解压后自动清理</div>
              <div class="text-white/30 text-[10px] uppercase tracking-widest">成功后自动将源文件移至回收站</div>
            </div>
            <div class="w-12 h-6 rounded-full border border-white/20 transition-all p-1"
                 :class="appStore.settings.autoDeleteSource ? 'bg-blue-500/30 border-blue-500' : 'bg-black/50'">
              <div class="w-4 h-4 rounded-full bg-white transition-all shadow-md"
                   :class="appStore.settings.autoDeleteSource ? 'translate-x-6' : ''"></div>
            </div>
          </div>
          
          <div class="w-full h-px bg-white/5"></div>

          <div class="flex items-center justify-between group cursor-pointer">
            <div>
              <div class="text-white font-medium mb-1 group-hover:text-blue-400 transition-colors">右键菜单集成</div>
              <div class="text-white/30 text-[10px] uppercase tracking-widest">在文件管理器中显示快捷操作</div>
            </div>
            <button class="px-4 py-1.5 rounded-lg bg-white/10 text-white/60 text-[10px] font-bold uppercase hover:bg-white/20 transition-all">
              配置
            </button>
          </div>
        </div>
      </section>

      <!-- 高级安全 -->
      <section class="p-8 rounded-3xl border border-red-500/10 backdrop-blur-xl bg-red-500/5 shadow-2xl relative overflow-hidden">
        <div class="absolute -right-10 -top-10 text-9xl text-red-500/5 pi pi-bolt pointer-events-none"></div>
        <h2 class="text-xs font-black text-red-400/80 uppercase tracking-widest mb-6">高级安全与破解</h2>
        <div class="space-y-6 relative z-10">
          <div class="flex items-center justify-between group cursor-pointer" @click="toggleBruteForce">
            <div>
              <div class="text-white font-medium mb-1 group-hover:text-red-400 transition-colors flex items-center gap-2">
                暴力破解引擎
                <span class="px-2 py-0.5 rounded bg-red-500/20 text-red-400 text-[8px] font-black uppercase tracking-widest">Danger</span>
              </div>
              <div class="text-white/30 text-[10px] uppercase tracking-widest">当所有已知密码失效时启动穷举算法</div>
            </div>
            <div class="w-12 h-6 rounded-full border transition-all p-1"
                 :class="appStore.settings.enableBruteForce ? 'bg-red-500/30 border-red-500' : 'bg-black/50 border-white/20'">
              <div class="w-4 h-4 rounded-full bg-white transition-all shadow-md"
                   :class="appStore.settings.enableBruteForce ? 'translate-x-6' : ''"></div>
            </div>
          </div>

          <transition name="expand">
            <div v-if="appStore.settings.enableBruteForce" class="pl-4 border-l-2 border-red-500/30 space-y-4">
              <div class="flex justify-between items-center">
                 <span class="text-xs text-white/60">外部字典库 (.txt)</span>
                 <button class="text-[10px] text-blue-400 hover:text-blue-300 font-bold uppercase tracking-widest">导入字典</button>
              </div>
              <div class="p-3 rounded-xl bg-black/40 border border-white/5 flex justify-between items-center text-xs">
                 <span class="text-white/40 font-mono">rockyou.txt (14.3 MB)</span>
                 <i class="pi pi-check-circle text-green-500"></i>
              </div>
            </div>
          </transition>
        </div>
      </section>

      <!-- 数据库维护 -->
      <section class="p-8 rounded-3xl border border-white/10 backdrop-blur-xl bg-white/5 shadow-2xl">
        <h2 class="text-xs font-black text-white/50 uppercase tracking-widest mb-6">系统维护</h2>
        <div class="flex gap-4">
           <button class="flex-1 py-3 rounded-xl border border-blue-500/30 bg-blue-500/10 text-blue-400 text-xs font-bold uppercase tracking-widest hover:bg-blue-500/20 transition-all flex items-center justify-center gap-2">
             <i class="pi pi-database"></i> 优化数据库
           </button>
           <button class="flex-1 py-3 rounded-xl border border-white/10 bg-white/5 text-white/60 text-xs font-bold uppercase tracking-widest hover:bg-white/10 transition-all flex items-center justify-center gap-2">
             <i class="pi pi-shield"></i> 健康诊断
           </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  background: radial-gradient(circle at 100% 100%, rgba(255, 255, 255, 0.02) 0%, transparent 50%);
}

.expand-enter-active, .expand-leave-active {
  transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
  max-height: 200px;
  opacity: 1;
}
.expand-enter-from, .expand-leave-to {
  max-height: 0;
  opacity: 0;
  overflow: hidden;
}
</style>
