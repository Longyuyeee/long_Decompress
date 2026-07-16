<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { open } from '@tauri-apps/api/dialog'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()

interface ChecksumResult {
  path: string
  fileName: string
  algorithm: string
  checksum: string
  status: 'success' | 'pending' | 'error'
  error?: string
}

const selectedFiles = ref<string[]>([])
const selectedAlgorithm = ref<'crc32' | 'md5' | 'sha256'>('sha256')
const checksumResults = ref<ChecksumResult[]>([])
const isCalculating = ref(false)
const verifyMode = ref(false)
const verifyResult = ref<{ valid: boolean; message: string } | null>(null)

const algorithms = [
  { value: 'crc32', label: 'CRC32', description: '快速校验' },
  { value: 'md5', label: 'MD5', description: '中等强度' },
  { value: 'sha256', label: 'SHA256', description: '高强度（推荐）' },
]

const selectFiles = async () => {
  try {
    const files = await open({
      multiple: true,
      title: appStore.t('integrity.select_files', '选择文件')
    })
    if (files) {
      selectedFiles.value = Array.isArray(files) ? files : [files]
      checksumResults.value = []
      verifyResult.value = null
    }
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

const calculateChecksums = async () => {
  if (selectedFiles.value.length === 0) {
    appStore.setError(appStore.t('integrity.no_files', '请先选择文件'))
    return
  }

  isCalculating.value = true
  checksumResults.value = selectedFiles.value.map(path => ({
    path,
    fileName: path.split(/[\\/]/).pop() || path,
    algorithm: selectedAlgorithm.value.toUpperCase(),
    checksum: '',
    status: 'pending'
  }))

  for (let i = 0; i < selectedFiles.value.length; i++) {
    const path = selectedFiles.value[i]
    try {
      const checksum = await tauriCommands.invoke<string>('calculate_checksum', {
        path,
        algorithm: selectedAlgorithm.value
      })
      checksumResults.value[i].checksum = checksum
      checksumResults.value[i].status = 'success'
    } catch (error: any) {
      checksumResults.value[i].status = 'error'
      checksumResults.value[i].error = String(error)
    }
  }

  isCalculating.value = false
  appStore.setSuccess(appStore.t('integrity.calc_complete', '校验和计算完成'))
}

const copyChecksum = async (checksum: string) => {
  try {
    await navigator.clipboard.writeText(checksum)
    appStore.setSuccess(appStore.t('integrity.copied', '已复制到剪贴板'))
  } catch {
    appStore.setError(appStore.t('integrity.copy_failed', '复制失败'))
  }
}

const exportChecksumFile = async () => {
  if (checksumResults.value.length === 0) {
    appStore.setError(appStore.t('integrity.no_results', '没有可导出的结果'))
    return
  }

  try {
    const firstFile = selectedFiles.value[0]
    const dir = firstFile.substring(0, Math.max(firstFile.lastIndexOf('/'), firstFile.lastIndexOf('\\')))
    const ext = selectedAlgorithm.value === 'crc32' ? '.sfv' : `.${selectedAlgorithm.value}`

    const savePath = await open({
      directory: false,
      multiple: false,
      title: appStore.t('integrity.export', '导出校验文件'),
      defaultPath: `${dir}/checksums${ext}`,
      filters: [{
        name: `${selectedAlgorithm.value.toUpperCase()} Checksum File`,
        extensions: [selectedAlgorithm.value]
      }]
    }) as string | null

    if (savePath) {
      await tauriCommands.invoke('export_checksum_file', {
        path: savePath,
        results: checksumResults.value.map(r => ({
          file_name: r.fileName,
          checksum: r.checksum
        })),
        algorithm: selectedAlgorithm.value
      })
      appStore.setSuccess(appStore.t('integrity.exported', '校验文件已导出'))
    }
  } catch (error) {
    appStore.setError(appStore.t('integrity.export_failed', '导出失败'))
  }
}

const selectChecksumFile = async () => {
  try {
    const file = await open({
      multiple: false,
      title: appStore.t('integrity.select_checksum', '选择校验文件'),
      filters: [{
        name: 'Checksum Files',
        extensions: ['md5', 'sha256', 'sfv']
      }]
    })
    if (file && typeof file === 'string') {
      await verifyChecksumFile(file)
    }
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

const verifyChecksumFile = async (checksumPath: string) => {
  isCalculating.value = true
  verifyResult.value = null

  try {
    const result = await tauriCommands.invoke<{ valid: boolean; message: string }>('verify_checksum_file', {
      checksumPath
    })
    verifyResult.value = result
    if (result.valid) {
      appStore.setSuccess(appStore.t('integrity.verify_success', '✓ 校验通过'))
    } else {
      appStore.setError(appStore.t('integrity.verify_failed', '✗ 校验失败'))
    }
  } catch (error: any) {
    verifyResult.value = {
      valid: false,
      message: String(error)
    }
    appStore.setError(appStore.t('integrity.verify_error', '校验出错'))
  } finally {
    isCalculating.value = false
  }
}

const clearResults = () => {
  selectedFiles.value = []
  checksumResults.value = []
  verifyResult.value = null
}
</script>

<template>
  <div class="flex flex-col h-full p-responsive p-8 transition-colors duration-700">
    <!-- 顶部标题栏 -->
    <header class="shrink-0 mb-8">
      <h1 class="text-4xl font-black text-content tracking-tighter mb-2">{{ appStore.t('integrity.title') }}</h1>
      <p class="text-muted text-[0.625rem] font-bold uppercase tracking-[0.3em] ml-1">{{ appStore.t('integrity.subtitle') }}</p>
    </header>

    <!-- 主内容区 -->
    <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-20">
      <div class="max-w-5xl space-y-8">
        <!-- 模式切换 -->
        <div class="grid grid-cols-2 gap-4">
          <button
            @click="verifyMode = false; clearResults()"
            class="aero-card p-8 text-left transition-all hover:scale-[1.02]"
            :class="!verifyMode ? 'ring-2 ring-primary' : ''"
          >
            <div class="text-4xl mb-3">🔢</div>
            <div class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('integrity.mode.calculate') }}</div>
            <div class="text-[0.5625rem] text-muted mt-2 uppercase tracking-tighter">计算文件的校验和</div>
          </button>
          <button
            @click="verifyMode = true; clearResults()"
            class="aero-card p-8 text-left transition-all hover:scale-[1.02]"
            :class="verifyMode ? 'ring-2 ring-primary' : ''"
          >
            <div class="text-4xl mb-3">✓</div>
            <div class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('integrity.mode.verify') }}</div>
            <div class="text-[0.5625rem] text-muted mt-2 uppercase tracking-tighter">验证校验文件的正确性</div>
          </button>
        </div>

        <!-- 计算模式 -->
        <div v-if="!verifyMode" class="space-y-8">
          <!-- 选择算法 -->
          <section class="aero-card p-10">
            <h2 class="text-[0.625rem] font-black text-content uppercase tracking-[0.3em] mb-6">
              {{ appStore.t('integrity.algorithm') }}
            </h2>
            <div class="grid grid-cols-3 gap-4">
              <button
                v-for="algo in algorithms"
                :key="algo.value"
                @click="selectedAlgorithm = algo.value as any"
                class="p-6 rounded-2xl border-2 transition-all text-left hover:scale-[1.02]"
                :class="selectedAlgorithm === algo.value
                  ? 'bg-primary/10 border-primary shadow-lg'
                  : 'bg-input/30 border-subtle hover:border-primary/50'"
              >
                <div class="text-sm font-black text-content uppercase tracking-widest">{{ algo.label }}</div>
                <div class="text-[0.5625rem] text-muted mt-2 uppercase tracking-tighter">{{ algo.description }}</div>
              </button>
            </div>
          </section>

          <!-- 选择文件 -->
          <section class="aero-card p-10">
            <h2 class="text-[0.625rem] font-black text-content uppercase tracking-[0.3em] mb-6">
              {{ appStore.t('integrity.files') }}
            </h2>
            <button
              @click="selectFiles"
              class="w-full px-10 py-12 rounded-2xl border-2 border-dashed border-primary/30 hover:border-primary hover:bg-primary/5 transition-all text-center group"
            >
              <div class="text-5xl mb-4 group-hover:scale-110 transition-transform">📁</div>
              <div class="text-sm font-black text-content uppercase tracking-widest">
                {{ selectedFiles.length > 0
                  ? `已选择 ${selectedFiles.length} 个文件`
                  : appStore.t('integrity.select_files') }}
              </div>
            </button>

            <!-- 操作按钮 -->
            <div class="flex gap-4 mt-6">
              <button
                @click="calculateChecksums"
                :disabled="selectedFiles.length === 0 || isCalculating"
                class="flex-1 px-8 py-4 rounded-2xl bg-primary text-white font-black uppercase tracking-widest text-sm hover:bg-primary/90 disabled:opacity-40 disabled:cursor-not-allowed transition-all shadow-lg hover:shadow-xl disabled:shadow-none"
              >
                {{ isCalculating ? appStore.t('integrity.calculating') : appStore.t('integrity.calculate') }}
              </button>
              <button
                v-if="checksumResults.length > 0"
                @click="exportChecksumFile"
                class="px-8 py-4 rounded-2xl bg-input/30 border-2 border-subtle text-content font-black uppercase tracking-widest text-sm hover:border-primary transition-all"
              >
                {{ appStore.t('integrity.export') }}
              </button>
            </div>
          </section>

          <!-- 结果列表 -->
          <section v-if="checksumResults.length > 0" class="aero-card p-10">
            <h2 class="text-[0.625rem] font-black text-content uppercase tracking-[0.3em] mb-6">
              {{ appStore.t('integrity.results') }}
            </h2>
            <div class="space-y-3">
              <div
                v-for="result in checksumResults"
                :key="result.path"
                class="p-6 rounded-2xl bg-input/30 border border-subtle hover:border-primary/50 transition-all"
              >
                <div class="flex items-start justify-between gap-4">
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-black text-content truncate uppercase tracking-widest">{{ result.fileName }}</div>
                    <div class="mt-3 flex items-center gap-3">
                      <span class="text-[0.5625rem] text-muted uppercase tracking-widest font-bold">{{ result.algorithm }}:</span>
                      <code class="text-xs font-mono text-primary">{{ result.checksum || '计算中...' }}</code>
                    </div>
                  </div>
                  <div class="flex items-center gap-3 shrink-0">
                    <span
                      v-if="result.status === 'success'"
                      class="text-green-500 text-2xl"
                    >✓</span>
                    <span
                      v-else-if="result.status === 'error'"
                      class="text-red-500 text-2xl"
                    >✗</span>
                    <button
                      v-if="result.status === 'success'"
                      @click="copyChecksum(result.checksum)"
                      class="px-4 py-2 rounded-xl bg-primary/10 hover:bg-primary/20 text-primary text-[0.625rem] font-black uppercase tracking-widest transition-all"
                    >
                      {{ appStore.t('integrity.copy') }}
                    </button>
                  </div>
                </div>
                <div v-if="result.error" class="mt-3 text-[0.5625rem] text-red-500 uppercase tracking-tighter">
                  {{ result.error }}
                </div>
              </div>
            </div>
          </section>
        </div>

        <!-- 验证模式 -->
        <div v-else class="space-y-8">
          <section class="aero-card p-10">
            <button
              @click="selectChecksumFile"
              :disabled="isCalculating"
              class="w-full px-10 py-16 rounded-2xl border-2 border-dashed border-primary/30 hover:border-primary hover:bg-primary/5 transition-all text-center group"
            >
              <div class="text-6xl mb-6 group-hover:scale-110 transition-transform">✓</div>
              <div class="text-base font-black text-content uppercase tracking-widest">
                {{ appStore.t('integrity.select_checksum') }}
              </div>
              <div class="text-[0.5625rem] text-muted mt-3 uppercase tracking-tighter">
                支持 .md5, .sha256, .sfv 格式
              </div>
            </button>
          </section>

          <!-- 验证结果 -->
          <section v-if="verifyResult" class="aero-card p-10 border-2"
            :class="verifyResult.valid
              ? 'bg-green-500/5 border-green-500'
              : 'bg-red-500/5 border-red-500'"
          >
            <div class="flex items-center gap-6">
              <span class="text-6xl shrink-0">{{ verifyResult.valid ? '✓' : '✗' }}</span>
              <div class="flex-1">
                <div class="text-lg font-black uppercase tracking-widest"
                  :class="verifyResult.valid ? 'text-green-500' : 'text-red-500'"
                >
                  {{ verifyResult.valid ? appStore.t('integrity.verify_success', '校验通过') : appStore.t('integrity.verify_failed', '校验失败') }}
                </div>
                <div class="text-sm text-muted mt-1">{{ verifyResult.message }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
