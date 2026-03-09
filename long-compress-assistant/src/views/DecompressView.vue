<template>
  <div class="max-w-7xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-4 sm:mb-6 lg:mb-8">
      <h1 class="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">文件解压</h1>
      <p class="text-gray-600 dark:text-gray-400 text-sm sm:text-base">选择文件并配置解压选项</p>
    </div>

    <main class="grid grid-cols-1 lg:grid-cols-3 gap-3 xs:gap-4 sm:gap-6">
      <!-- 左侧：文件选择和配置 -->
      <div class="lg:col-span-2 space-y-4 xs:space-y-6">
        <!-- 文件选择组件 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">选择文件</h2>

          <EnhancedFileDropzone
            :multiple="true"
            :maxSize="1024 * 1024 * 1024" 
            :maxFiles="20"
            :useTauriDialog="true"
            :showPreview="false"
            @files-selected="handleFilesSelected"
            @file-removed="handleFileRemoved"
            @error="handleFileError"
            class="mb-4"
          />

          <!-- 文件统计信息 -->
          <div v-if="selectedFiles.length > 0" class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
            <div class="flex items-center justify-between mb-4">
              <h3 class="font-medium text-gray-900 dark:text-white">已选择文件 ({{ selectedFiles.length }})</h3>
              <button
                @click="clearAllFiles"
                class="text-sm text-gray-500 hover:text-red-500 transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 rounded p-1"
              >
                <i class="pi pi-trash mr-1"></i> 清空所有
              </button>
            </div>

            <div class="grid grid-cols-1 xs:grid-cols-2 gap-2 xs:gap-3">
              <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                <div class="flex items-center">
                  <i class="pi pi-file text-gray-500 mr-3"></i>
                  <div class="flex-1">
                    <p class="font-medium text-gray-900 dark:text-white">文件总数</p>
                    <p class="text-2xl font-bold text-primary">{{ selectedFiles.length }}</p>
                  </div>
                </div>
              </div>

              <div class="p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                <div class="flex items-center">
                  <i class="pi pi-database text-gray-500 mr-3"></i>
                  <div class="flex-1">
                    <p class="font-medium text-gray-900 dark:text-white">总大小</p>
                    <p class="text-2xl font-bold text-primary">{{ formatTotalSize() }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 解压配置 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">解压配置</h2>
          <DecompressSettingsPanel
            v-model="decompressSettings"
            :is-processing="isProcessing"
            @settings-change="handleSettingsChange"
          />
        </div>
      </div>

      <!-- 右侧：操作面板 -->
      <div class="space-y-4 xs:space-y-6">
        <!-- 开始解压按钮 -->
        <div class="glass-card">
          <button @click="startDecompress"
                  :disabled="!canStart || isProcessing"
                  class="w-full glass-button-primary py-4 text-lg font-semibold flex items-center justify-center"
                  :class="{ 'opacity-50 cursor-not-allowed': !canStart || isProcessing }">
            <i v-if="isProcessing" class="pi pi-spin pi-spinner mr-3"></i>
            <i v-else class="pi pi-play mr-3"></i>
            {{ isProcessing ? '正在处理任务...' : '开始批量解压' }}
          </button>
          <p class="text-gray-500 dark:text-gray-400 text-sm mt-3 text-center">
            {{ canStart ? '将使用智能密码本自动匹配解压' : '请先选择要解压的文件' }}
          </p>
        </div>

        <!-- 进度显示 -->
        <div class="glass-card" v-if="isProcessing || progressTasks.some(task => task.status !== 'pending')">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">执行队列</h3>

          <div class="space-y-4 max-h-80 overflow-y-auto pr-2">
            <div v-for="task in progressTasks" :key="task.id" class="space-y-2 p-3 rounded-xl border border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-900/50">
              <div class="flex justify-between items-center text-sm">
                <div class="flex items-center min-w-0 flex-1">
                  <span class="text-gray-700 dark:text-gray-300 truncate mr-2 font-medium">{{ task.fileName }}</span>
                  <span v-if="task.status === 'completed'" class="text-[10px] px-2 py-0.5 rounded-full bg-green-500/10 text-green-500">完成</span>
                  <span v-else-if="task.status === 'failed'" class="text-[10px] px-2 py-0.5 rounded-full bg-red-500/10 text-red-500">失败</span>
                  <span v-else-if="task.status === 'processing'" class="text-[10px] px-2 py-0.5 rounded-full bg-primary/10 text-primary animate-pulse">
                    {{ task.retryCount > 0 ? `尝试密码 #${task.retryCount}` : '解压中' }}
                  </span>
                </div>
                <span class="font-mono text-xs">{{ task.progress }}%</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5 overflow-hidden">
                <div
                  class="h-full transition-all duration-500"
                  :class="{
                    'bg-primary': task.status === 'processing',
                    'bg-green-500': task.status === 'completed',
                    'bg-red-500': task.status === 'failed'
                  }"
                  :style="{ width: task.progress + '%' }"
                ></div>
              </div>
              <div v-if="task.error" class="text-[10px] text-red-400 mt-1 flex items-start">
                <i class="pi pi-exclamation-triangle mr-1 mt-0.5"></i>
                <span class="flex-1">{{ task.error }}</span>
              </div>
            </div>
          </div>

          <div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 space-y-3">
            <div>
              <div class="flex justify-between text-xs mb-1">
                <span class="text-gray-500">总体进度</span>
                <span class="font-bold">{{ totalProgress }}%</span>
              </div>
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div class="bg-primary h-full rounded-full transition-all duration-1000" :style="{ width: totalProgress + '%' }"></div>
              </div>
            </div>
          </div>
        </div>

        <!-- 智能说明 -->
        <div class="glass-card p-4 bg-primary/5 border-primary/10">
          <div class="flex items-start gap-3">
            <div class="p-2 bg-primary/10 rounded-lg">
              <i class="pi pi-sparkles text-primary"></i>
            </div>
            <div>
              <h4 class="text-sm font-bold text-primary mb-1">智能自动化已启用</h4>
              <p class="text-xs text-gray-500 leading-relaxed">
                系统会自动检测加密文件，并基于文件名、标签和历史记录从您的密码本中提取最可能的候选密码进行静默尝试。
              </p>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { usePasswordStore } from '@/stores/password'
import { useUIStore } from '@/stores/ui'
import { useConfigStore } from '@/stores/config'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import DecompressSettingsPanel from '@/components/ui/DecompressSettingsPanel.vue'
import type { FileItem } from '@/components/ui/EnhancedFileDropzone.vue'
import type { DecompressSettingsPanelSettings } from '@/components/ui/DecompressSettingsPanel.vue'

const passwordStore = usePasswordStore()
const uiStore = useUIStore()
const configStore = useConfigStore()

// 状态
const selectedFiles = ref<FileItem[]>([])
const isProcessing = ref(false)
const processedFiles = ref(0)
const currentFileProgress = ref(0)

const decompressSettings = ref<DecompressSettingsPanelSettings>({
  outputPath: '',
  password: '',
  options: {
    keepStructure: true,
    overwriteStrategy: 'ask',
    deleteAfter: false,
    preserveTimestamps: true,
    skipCorrupted: false,
    extractOnlyNewer: false,
    createSubdirectory: false,
    fileFilter: ''
  },
  passwordOptions: {
    rememberForSession: false,
    autoTryCommon: true,
    maxAttempts: 5
  }
})

// 增强的任务状态(TSK-103)
const progressTasks = ref<Array<{
  id: string | number
  fileName: string
  filePath: string
  progress: number
  status: 'pending' | 'processing' | 'completed' | 'failed'
  error?: string
  retryCount: number
  candidatePasswords: string[]
}>>([])

let unlistenProgress: UnlistenFn | null = null;
let unlistenStatus: UnlistenFn | null = null;

onMounted(async () => {
  // 1. 监听进度
  unlistenProgress = await listen<{ task_id: string, progress: number }>('task_progress', (event) => {
    const task = progressTasks.value.find(t => t.id === event.payload.task_id);
    if (task) {
      task.progress = Math.round(event.payload.progress * 100);
    }
  });

  // 2. 监听状态与自动重试 (核心逻辑)
  unlistenStatus = await listen<{ task_id: string, status: string }>('task_status', async (event) => {
    const taskIndex = progressTasks.value.findIndex(t => t.id === event.payload.task_id);
    if (taskIndex === -1) return;
    
    const task = progressTasks.value[taskIndex];
    const statusStr = event.payload.status.toLowerCase();

    if (statusStr.includes('completed')) {
      task.status = 'completed';
      task.progress = 100;
      uiStore.showSuccess(`文件 ${task.fileName} 解压成功`);
      
      // 系统通知 (TSK-302)
      try {
        await invoke('send_task_completed_notification', { taskName: task.fileName });
      } catch (e) {}

      checkAllCompleted();
    } 
    else if (statusStr.includes('failed') || statusStr.includes('error')) {
      // 检查是否是因为密码错误引起的失败
      const isPasswordError = true; 
      
      if (isPasswordError && task.candidatePasswords.length > 0 && task.retryCount < decompressSettings.value.passwordOptions.maxAttempts) {
        const nextPassword = task.candidatePasswords.shift();
        task.retryCount++;
        console.log(`[智能重试] 文件: ${task.fileName}, 正在尝试第 ${task.retryCount} 个候选密码...`);
        
        try {
          const newTaskId = await invoke<string>('add_extraction_task', { 
            request: {
              source_file: task.filePath,
              output_dir: decompressSettings.value.outputPath || null,
              password: nextPassword,
              priority: 'High'
            }
          });
          task.id = newTaskId; // 更新 ID 以便后续监听
          task.status = 'processing';
          task.progress = 0;
        } catch (e) {
          task.status = 'failed';
          task.error = `重试提交失败: ${e}`;
          uiStore.showError(`文件 ${task.fileName} 解压失败`);
          checkAllCompleted();
        }
      } else {
        task.status = 'failed';
        task.error = task.retryCount > 0 ? `尝试了所有密码均失败` : '解压失败，可能需要密码或文件损坏';
        uiStore.showError(`文件 ${task.fileName} 解压失败`);
        checkAllCompleted();
      }
    } else if (statusStr.includes('running')) {
      task.status = 'processing';
    }
  });
});

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress();
  if (unlistenStatus) unlistenStatus();
});

const checkAllCompleted = () => {
  const active = progressTasks.value.filter(t => t.status === 'pending' || t.status === 'processing');
  if (active.length === 0) {
    isProcessing.value = false;
  }
};

const startDecompress = async () => {
  if (!canStart.value) return;
  
  isProcessing.value = true;
  processedFiles.value = 0;

  // 确保密码库已解锁 (TSK-201 联动)
  if (decompressSettings.value.passwordOptions.autoTryCommon && !passwordStore.isUnlocked) {
    const wantUnlock = window.confirm('自动匹配密码需要解锁密码库。是否现在解锁？');
    if (wantUnlock) {
      uiStore.showInfo('请在侧边栏导航到密码本页面进行解锁');
      isProcessing.value = false;
      return;
    }
  }

  for (let i = 0; i < selectedFiles.value.length; i++) {
    const file = selectedFiles.value[i];
    
    // 初始化候选密码列表
    let candidates: string[] = [];
    if (decompressSettings.value.passwordOptions.autoTryCommon) {
      candidates = passwordStore.findCandidatePasswords(file.name);
    }

    const taskIndex = progressTasks.value.findIndex(t => t.fileName === file.name && t.status === 'pending');
    if (taskIndex === -1) continue;
    
    const task = progressTasks.value[taskIndex];
    task.filePath = file.path;
    task.candidatePasswords = candidates;

    // 首轮尝试：使用手动输入的密码，或者直接开始
    try {
      const taskId = await invoke<string>('add_extraction_task', { 
        request: {
          source_file: file.path,
          output_dir: decompressSettings.value.outputPath || null,
          password: decompressSettings.value.password || null,
          priority: 'Medium'
        }
      });
      task.id = taskId;
    } catch (error) {
      task.status = 'failed';
      task.error = String(error);
      uiStore.showError(`提交任务失败: ${error}`);
      checkAllCompleted();
    }
  }
};

// 计算属性
const canStart = computed(() => selectedFiles.value.length > 0)
const totalProgress = computed(() => {
  if (progressTasks.value.length === 0) return 0
  const total = progressTasks.value.reduce((acc, t) => acc + t.progress, 0)
  return Math.round(total / progressTasks.value.length)
})

const handleFilesSelected = (files: FileItem[]) => {
  selectedFiles.value = [...selectedFiles.value, ...files]
  files.forEach(file => {
    progressTasks.value.push({
      id: Math.random().toString(36),
      fileName: file.name,
      filePath: file.path,
      progress: 0,
      status: 'pending',
      retryCount: 0,
      candidatePasswords: []
    })
  })
}

const handleFileRemoved = (fileId: string) => {
  const removed = selectedFiles.value.find(f => f.id === fileId);
  if (removed) {
    progressTasks.value = progressTasks.value.filter(t => t.fileName !== removed.name);
  }
  selectedFiles.value = selectedFiles.value.filter(f => f.id !== fileId);
}

const clearAllFiles = () => {
  selectedFiles.value = [];
  progressTasks.value = [];
}

const formatTotalSize = () => {
  const bytes = selectedFiles.value.reduce((sum, f) => sum + f.size, 0);
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

const handleFileError = (e: any) => {
  console.error(e);
  uiStore.showError(String(e));
}
const handleSettingsChange = (s: any) => {};
</script>

<style scoped>
/* 苹果风格微调 */
::-webkit-scrollbar { width: 4px; }
::-webkit-scrollbar-thumb { @apply bg-gray-300 dark:bg-gray-700 rounded-full; }
</style>
