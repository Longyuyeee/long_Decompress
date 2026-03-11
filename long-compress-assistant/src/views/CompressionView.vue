<script setup lang="ts">
import { ref } from 'vue'
import { useCompressionStore } from '@/stores/compression'
import CompressionAeroTable from '@/components/tasks/CompressionAeroTable.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import { useTauriCommands } from '@/composables/useTauriCommands'

const compressionStore = useCompressionStore()
const tauriCommands = useTauriCommands()

const globalSettings = ref({
  format: 'zip',
  level: 5,
  password: ''
})

const onFilesSelected = (files: any[]) => {
  const paths = files.map(f => f.path)
  compressionStore.selectedFiles.push(...paths)
}

const startCompression = async () => {
  // 实现批量压缩逻辑
  console.log('开始压缩...', compressionStore.groups)
}
</script>

<template>
  <div class="compression-view p-8 min-h-screen">
    <!-- 极简页头 -->
    <header class="mb-10 flex justify-between items-end">
      <div>
        <h1 class="text-4xl font-black text-white tracking-tighter mb-2">压缩中心</h1>
        <p class="text-white/40 text-sm font-medium tracking-wide uppercase">Compression Workspace v2.1</p>
      </div>
    </header>

    <div class="grid grid-cols-1 gap-8">
      <!-- 拖拽感应区 (空状态) -->
      <section v-if="compressionStore.selectedFiles.length === 0 && compressionStore.groups.length === 0" 
               class="flex flex-col items-center justify-center py-20">
        <EnhancedFileDropzone @files-selected="onFilesSelected" class="w-full max-w-2xl" />
      </section>

      <!-- 磁吸表格区 -->
      <section v-else class="space-y-10">
        <CompressionAeroTable />
        
        <!-- 全局配置与启动 -->
        <div class="compression-footer glass-card p-6 rounded-3xl border border-white/10 sticky bottom-8 shadow-2xl backdrop-blur-3xl bg-white/5">
          <div class="flex items-center gap-12">
            <!-- 格式选择 -->
            <div class="flex-1">
              <div class="text-[10px] text-white/30 uppercase font-black tracking-widest mb-4">目标格式</div>
              <div class="flex gap-4">
                <button v-for="fmt in ['ZIP', '7Z', 'TAR']" :key="fmt"
                        @click="globalSettings.format = fmt.toLowerCase()"
                        class="px-6 py-2 rounded-xl border text-[10px] font-bold transition-all"
                        :class="globalSettings.format === fmt.toLowerCase() 
                          ? 'bg-blue-500/30 border-blue-500/50 text-white' 
                          : 'bg-white/5 border-white/10 text-white/40 hover:border-white/20'">
                  {{ fmt }}
                </button>
              </div>
            </div>

            <!-- 压缩等级 -->
            <div class="flex-1">
              <div class="flex justify-between items-center mb-4">
                <div class="text-[10px] text-white/30 uppercase font-black tracking-widest">压缩等级</div>
                <div class="text-xs text-blue-400 font-mono">{{ globalSettings.level }}</div>
              </div>
              <input type="range" min="1" max="9" v-model="globalSettings.level" 
                     class="w-full h-1 bg-white/10 rounded-full appearance-none cursor-pointer accent-blue-500">
            </div>

            <!-- 启动按钮 (Morphing 准备) -->
            <div class="flex-none">
              <button @click="startCompression" 
                      class="px-12 py-4 rounded-2xl bg-white text-black font-black uppercase text-sm tracking-widest hover:scale-105 active:scale-95 transition-all shadow-[0_10px_30px_rgba(255,255,255,0.2)]">
                开始压缩
              </button>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.compression-view {
  background: radial-gradient(circle at 100% 0%, rgba(139, 92, 246, 0.05) 0%, transparent 50%),
              radial-gradient(circle at 0% 100%, rgba(59, 130, 246, 0.05) 0%, transparent 50%);
}

/* 简单的滑块样式 */
input[type='range']::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  background: white;
  border-radius: 50%;
  box-shadow: 0 0 10px rgba(59, 130, 246, 0.5);
}
</style>
