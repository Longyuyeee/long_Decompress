<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import { useTauriCommands } from '@/composables/useTauriCommands'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'

const appStore = useAppStore()
const compressionStore = useCompressionStore()
const tauriCommands = useTauriCommands()

const selectedRows = ref<Set<string>>(new Set())

const onFilesSelected = (files: any[]) => {
  files.forEach(f => {
    // 检查是否已经存在
    if (!compressionStore.selectedFiles.some(existing => existing.path === f.path)) {
      compressionStore.selectedFiles.push({
        name: f.name,
        path: f.path,
        size: f.size || 0,
        type: f.type || 'file',
        isDirectory: f.isDirectory || false
      })
    }
  })
}

const toggleSelection = (path: string) => {
  if (selectedRows.value.has(path)) selectedRows.value.delete(path)
  else selectedRows.value.add(path)
}

const handleCreateGroup = () => {
  if (selectedRows.value.size > 0) {
    compressionStore.createGroup(Array.from(selectedRows.value))
    selectedRows.value.clear()
  }
}

const handleCompress = async () => {
  if (compressionStore.groups.length === 0 && compressionStore.selectedFiles.length === 0) return
  
  appStore.successMessage = appStore.t('common.success')
  // 实际压缩逻辑对接后端...
}

const totalPayload = computed(() => {
  return compressionStore.selectedFiles.length + compressionStore.groups.reduce((acc, g) => acc + g.files.length, 0)
})
</script>

<template>
  <div class="compression-view p-responsive p-8 h-screen flex flex-col gap-8 transition-colors duration-700 overflow-hidden relative">
    <header class="flex justify-between items-center shrink-0">
      <div>
        <h1 class="text-4xl font-black text-content tracking-tighter mb-1">{{ appStore.t('nav.compress') }}</h1>
        <p class="text-muted text-[10px] font-bold uppercase tracking-[0.3em] ml-1">{{ appStore.t('app.tagline') }}</p>
      </div>
      
      <div class="flex items-center gap-4">
        <!-- 磁吸成组按钮 (浮现式) -->
        <transition name="pop">
          <button 
            v-if="selectedRows.size > 0"
            @click="handleCreateGroup"
            class="h-10 px-6 rounded-xl bg-primary text-white text-[10px] font-black uppercase tracking-widest shadow-lg shadow-primary/20 hover:scale-105 transition-all flex items-center gap-2"
          >
            <i class="pi pi-box animate-bounce"></i>
            {{ appStore.t('compress.create_group') }} ({{ selectedRows.size }})
          </button>
        </transition>

        <button 
          v-if="totalPayload > 0"
          @click="handleCompress"
          class="h-10 px-8 rounded-xl bg-input border border-subtle text-content text-[10px] font-black uppercase tracking-widest hover:bg-primary hover:text-white transition-all shadow-sm flex items-center gap-2"
        >
          {{ appStore.t('compress.start') }}
        </button>
      </div>
    </header>

    <!-- 主工作区 (减小 mb 以使区域向下延展) -->
    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col mb-6 relative border border-subtle bg-card/40 shadow-2xl">
      <div v-if="totalPayload > 0" class="flex-1 overflow-y-auto custom-scrollbar p-6 space-y-6">
        
        <!-- 1. 压缩组列表 -->
        <div v-for="group in compressionStore.groups" :key="group.id" 
             class="group-container rounded-[2rem] border transition-all duration-500 overflow-hidden"
             :class="group.expanded ? 'bg-input/40 border-primary/30 shadow-lg' : 'bg-input/20 border-subtle hover:border-primary/20'"
             :style="{ borderColor: group.expanded ? group.themeColor : '' }">
          
          <!-- 组头部 -->
          <div class="flex items-center px-8 py-5 cursor-pointer group/header" @click="group.expanded = !group.expanded">
            <div class="w-10 h-10 rounded-xl flex items-center justify-center mr-6 shadow-sm transition-transform group-hover/header:rotate-6"
                 :style="{ backgroundColor: `${group.themeColor}20`, color: group.themeColor, border: `1px solid ${group.themeColor}40` }">
              <i class="pi pi-briefcase text-sm"></i>
            </div>
            
            <div class="flex-1">
              <div class="text-sm font-black text-content tracking-tight">{{ group.name }}</div>
              <div class="flex items-center gap-2 mt-1">
                <span class="text-[9px] font-bold text-muted uppercase tracking-widest">{{ group.files.length }} {{ appStore.t('compress.group_count') }}</span>
                <div class="w-1 h-1 rounded-full bg-subtle"></div>
                <span class="text-[9px] font-mono text-primary font-black uppercase">{{ group.settings.format }}</span>
              </div>
            </div>

            <div class="flex items-center gap-4">
              <button @click.stop="compressionStore.dissolveGroup(group.id)" class="text-muted hover:text-red-500 transition-colors">
                <i class="pi pi-trash text-xs"></i>
              </button>
              <i class="pi transition-transform duration-500 text-muted text-[10px]" :class="group.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
            </div>
          </div>

          <!-- 组展开：独立配置面板 -->
          <transition name="slide-down">
            <div v-if="group.expanded" class="px-8 pb-8 pt-4 border-t border-subtle/30">
              <div class="mb-6">
                <h4 class="text-[8px] font-black text-muted uppercase tracking-widest mb-4">{{ appStore.t('compress.settings') }}</h4>
                <!-- 使用横向配置组件，适配该组 -->
                <CompressionSettingsPanel 
                  v-model="group.settings"
                />
              </div>
              
              <div class="space-y-2">
                <h4 class="text-[8px] font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('compress.group_files') }}</h4>
                <div v-for="file in group.files" :key="file.path" class="text-[10px] text-muted font-mono py-1 px-3 bg-card/40 rounded-lg border border-subtle/50 flex items-center justify-between">
                  <div class="flex items-center gap-2 overflow-hidden">
                    <i :class="file.isDirectory ? 'pi pi-folder text-primary/60' : 'pi pi-file text-muted/60'" class="text-[9px]"></i>
                    <span class="truncate">{{ file.name }}</span>
                  </div>
                  <span class="opacity-30 italic shrink-0 ml-4">{{ file.path }}</span>
                </div>
              </div>
            </div>
          </transition>
        </div>

        <!-- 2. 未分组文件列表 (待分配) -->
        <div v-if="compressionStore.selectedFiles.length > 0" class="space-y-3">
          <h3 class="text-[9px] font-black text-muted uppercase tracking-[0.3em] px-4">{{ appStore.t('compress.add_files') }}</h3>
          <div v-for="file in compressionStore.selectedFiles" :key="file.path" 
               @click="toggleSelection(file.path)"
               class="flex items-center justify-between px-8 py-4 rounded-2xl bg-input border border-subtle group/row hover:border-primary/30 transition-all cursor-pointer"
               :class="{ 'border-primary/50 bg-primary/5 shadow-inner': selectedRows.has(file.path) }">
            
            <div class="w-6 flex shrink-0">
              <div class="w-4 h-4 rounded border border-subtle flex items-center justify-center transition-all"
                   :class="selectedRows.has(file.path) ? 'bg-primary border-primary' : 'bg-card'">
                <i v-if="selectedRows.has(file.path)" class="pi pi-check text-[8px] text-white"></i>
              </div>
            </div>

            <div class="flex-1 min-w-[200px] overflow-hidden px-4 flex items-center gap-3">
              <div class="w-8 h-8 rounded-lg bg-card border border-subtle flex items-center justify-center shrink-0">
                <i :class="file.isDirectory ? 'pi pi-folder text-primary' : 'pi pi-file text-muted'" class="text-xs"></i>
              </div>
              <div class="overflow-hidden">
                <div class="text-content font-bold truncate text-xs tracking-tight group-hover/row:text-primary transition-colors">{{ file.name }}</div>
                <div class="text-[9px] text-muted font-mono mt-0.5 opacity-60 truncate">{{ file.path }}</div>
              </div>
            </div>

            <button @click.stop="compressionStore.selectedFiles = compressionStore.selectedFiles.filter(f => f.path !== file.path)" 
                    class="w-8 h-8 rounded-lg flex items-center justify-center text-dim hover:text-red-500 transition-all">
              <i class="pi pi-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- 3. 空状态 (完美垂直居中) -->
      <div v-else class="flex-1 flex items-center justify-center p-12 overflow-hidden">
        <div class="grid grid-cols-2 gap-8 w-full max-w-4xl">
          <!-- 文件夹拖入区 -->
          <EnhancedFileDropzone 
            @files-selected="onFilesSelected" 
            mode="folder"
            class="shadow-sm h-full" 
          />
          <!-- 文件拖入区 -->
          <EnhancedFileDropzone 
            @files-selected="onFilesSelected" 
            mode="file"
            :hint="appStore.t('compress.drop_file_hint')"
            class="shadow-sm h-full" 
          />
        </div>
      </div>

      <!-- 底部辅助区 -->
      <div v-if="totalPayload > 0" class="p-2 border-t border-subtle bg-input/10 grid grid-cols-2 gap-2">
        <EnhancedFileDropzone 
          @files-selected="onFilesSelected" 
          :compact="true" 
          mode="folder"
          class="w-full" 
        />
        <EnhancedFileDropzone 
          @files-selected="onFilesSelected" 
          :compact="true" 
          mode="file"
          class="w-full" 
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.compression-view {
  background: radial-gradient(circle at 100% 100%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.slide-down-enter-active, .slide-down-leave-active { transition: all 0.5s cubic-bezier(0.34, 1.56, 0.64, 1); }
.slide-down-enter-from, .slide-down-leave-to { opacity: 0; transform: translateY(-10px); }

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.8) translateY(20px); }
</style>
