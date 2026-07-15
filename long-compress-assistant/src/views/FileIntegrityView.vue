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
  <div class="flex flex-col h-full bg-background text-primary">
    <!-- 顶部标题栏 -->
    <div class="px-6 py-5 border-b border-subtle">
      <div class="flex items-center gap-3">
        <span class="text-2xl">🔐</span>
        <div>
          <h1 class="text-lg font-bold">{{ appStore.t('integrity.title', '文件完整性校验') }}</h1>
          <p class="text-xs text-muted mt-1">{{ appStore.t('integrity.subtitle', '计算和验证文件校验和') }}</p>
        </div>
      </div>
    </div>

    <!-- 主内容区 -->
    <div class="flex-1 overflow-auto p-6">
      <div class="max-w-4xl mx-auto space-y-6">
        <!-- 模式切换 -->
        <div class="flex gap-3">
          <button
            @click="verifyMode = false; clearResults()"
            class="flex-1 px-6 py-4 rounded-xl border transition-all"
            :class="!verifyMode
              ? 'bg-primary/10 border-primary text-primary'
              : 'bg-input/30 border-subtle text-muted hover:border-primary/50'"
          >
            <div class="text-2xl mb-2">🔢</div>
            <div class="text-sm font-bold">{{ appStore.t('integrity.mode.calculate', '计算校验和') }}</div>
          </button>
          <button
            @click="verifyMode = true; clearResults()"
            class="flex-1 px-6 py-4 rounded-xl border transition-all"
            :class="verifyMode
              ? 'bg-primary/10 border-primary text-primary'
              : 'bg-input/30 border-subtle text-muted hover:border-primary/50'"
          >
            <div class="text-2xl mb-2">✓</div>
            <div class="text-sm font-bold">{{ appStore.t('integrity.mode.verify', '验证校验文件') }}</div>
          </button>
        </div>

        <!-- 计算模式 -->
        <div v-if="!verifyMode" class="space-y-6">
          <!-- 选择算法 -->
          <div class="space-y-3">
            <label class="text-xs font-bold text-primary uppercase tracking-wider">
              {{ appStore.t('integrity.algorithm', '校验算法') }}
            </label>
            <div class="grid grid-cols-3 gap-3">
              <button
                v-for="algo in algorithms"
                :key="algo.value"
                @click="selectedAlgorithm = algo.value as any"
                class="px-4 py-3 rounded-xl border transition-all text-left"
                :class="selectedAlgorithm === algo.value
                  ? 'bg-primary/10 border-primary text-primary'
                  : 'bg-input/30 border-subtle text-muted hover:border-primary/50'"
              >
                <div class="text-sm font-bold">{{ algo.label }}</div>
                <div class="text-xs text-muted mt-1">{{ algo.description }}</div>
              </button>
            </div>
          </div>

          <!-- 选择文件 -->
          <div class="space-y-3">
            <label class="text-xs font-bold text-primary uppercase tracking-wider">
              {{ appStore.t('integrity.files', '文件') }}
            </label>
            <button
              @click="selectFiles"
              class="w-full px-6 py-4 rounded-xl border-2 border-dashed border-primary/30 hover:border-primary hover:bg-primary/5 transition-all text-center"
            >
              <div class="text-3xl mb-2">📁</div>
              <div class="text-sm font-bold text-primary">
                {{ selectedFiles.length > 0
                  ? `已选择 ${selectedFiles.length} 个文件`
                  : appStore.t('integrity.select_files', '点击选择文件') }}
              </div>
            </button>
          </div>

          <!-- 操作按钮 -->
          <div class="flex gap-3">
            <button
              @click="calculateChecksums"
              :disabled="selectedFiles.length === 0 || isCalculating"
              class="flex-1 px-6 py-3 rounded-xl bg-primary text-white font-bold hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {{ isCalculating ? '计算中...' : appStore.t('integrity.calculate', '计算校验和') }}
            </button>
            <button
              v-if="checksumResults.length > 0"
              @click="exportChecksumFile"
              class="px-6 py-3 rounded-xl bg-input/30 border border-subtle text-primary font-bold hover:border-primary transition-colors"
            >
              {{ appStore.t('integrity.export', '导出') }}
            </button>
          </div>

          <!-- 结果列表 -->
          <div v-if="checksumResults.length > 0" class="space-y-3">
            <label class="text-xs font-bold text-primary uppercase tracking-wider">
              {{ appStore.t('integrity.results', '结果') }}
            </label>
            <div class="space-y-2">
              <div
                v-for="result in checksumResults"
                :key="result.path"
                class="p-4 rounded-xl bg-input/30 border border-subtle"
              >
                <div class="flex items-start justify-between gap-3">
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-bold text-primary truncate">{{ result.fileName }}</div>
                    <div class="mt-2 flex items-center gap-2">
                      <span class="text-xs text-muted">{{ result.algorithm }}:</span>
                      <code class="text-xs font-mono text-primary">{{ result.checksum || '计算中...' }}</code>
                    </div>
                  </div>
                  <div class="flex items-center gap-2">
                    <span
                      v-if="result.status === 'success'"
                      class="text-green-500 text-xl"
                    >✓</span>
                    <span
                      v-else-if="result.status === 'error'"
                      class="text-red-500 text-xl"
                    >✗</span>
                    <button
                      v-if="result.status === 'success'"
                      @click="copyChecksum(result.checksum)"
                      class="px-3 py-1.5 rounded-lg bg-primary/10 hover:bg-primary/20 text-primary text-xs font-bold transition-colors"
                    >
                      {{ appStore.t('integrity.copy', '复制') }}
                    </button>
                  </div>
                </div>
                <div v-if="result.error" class="mt-2 text-xs text-red-500">
                  {{ result.error }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 验证模式 -->
        <div v-else class="space-y-6">
          <button
            @click="selectChecksumFile"
            :disabled="isCalculating"
            class="w-full px-6 py-8 rounded-xl border-2 border-dashed border-primary/30 hover:border-primary hover:bg-primary/5 transition-all text-center"
          >
            <div class="text-4xl mb-3">✓</div>
            <div class="text-base font-bold text-primary">
              {{ appStore.t('integrity.select_checksum', '选择校验文件进行验证') }}
            </div>
            <div class="text-xs text-muted mt-2">
              支持 .md5, .sha256, .sfv 格式
            </div>
          </button>

          <!-- 验证结果 -->
          <div v-if="verifyResult" class="p-6 rounded-xl border"
            :class="verifyResult.valid
              ? 'bg-green-500/10 border-green-500'
              : 'bg-red-500/10 border-red-500'"
          >
            <div class="flex items-center gap-3">
              <span class="text-3xl">{{ verifyResult.valid ? '✓' : '✗' }}</span>
              <div class="flex-1">
                <div class="text-base font-bold"
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
