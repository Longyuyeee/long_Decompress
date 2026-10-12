<script setup lang="ts">
import { open, save } from '@tauri-apps/api/dialog'
import { watch } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import { formatProgressPercent } from '@/utils/progress'
import ArchiveOutputSummary from './ArchiveOutputSummary.vue'
import { conversionFormats, useArchiveConversionStore } from '@/stores/archiveConversion'
const store = useArchiveConversionStore()
const queuedSelection = () => import.meta.env.VITE_DESKTOP_E2E === '1'
  ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
async function chooseSource() {
  try {
    const queued = queuedSelection()
    const selected = queued === undefined ? await open({ multiple: false, title: '选择原压缩包（分卷请选择首卷）' }) : queued
    if (typeof selected === 'string') store.selectSource(selected)
  } catch (error) { store.error = String(error) }
}
async function chooseOutput() {
  try {
    const queued = queuedSelection()
    const selected = queued === undefined ? await save({ title: '另存新压缩包', defaultPath: `转换结果.${store.format}` }) : queued
    if (typeof selected === 'string') store.outputPath = selected
  } catch (error) { store.error = String(error) }
}
watch(() => store.format, (format, old) => {
  if (store.outputPath.toLowerCase().endsWith(`.${old}`)) store.outputPath = store.outputPath.slice(0, -old.length) + format
  if (format !== '7z') store.encryptFilenames = false
  if (!store.supportsPassword) { store.password = ''; store.splitMiB = 0 }
})
</script>

<template>
  <Modal :visible="store.visible" :title="store.contextIntent === 'reconfigure' ? '修改密码 / 调整分卷' : '转换压缩包'" size="lg" @update:visible="store.visible = $event" @close="store.visible = false">
    <div data-testid="archive-conversion" class="space-y-4">
      <div class="flex items-center gap-2 text-sm text-muted">保留原包，生成新包
        <HelpHint label="转换说明">转换会先解压再重新压缩。需要足够空间同时保存展开内容和新包；不会覆盖已有输出。密码留空表示新包不加密。分卷请选择首卷。</HelpHint>
      </div>
      <section v-if="store.task?.status === 'completed' && store.task.archiveOutput" data-testid="conversion-result" class="rounded-xl border border-emerald-500/30 p-3 space-y-2">
        <p class="text-emerald-600 font-bold">转换完成 · 原压缩包已保留</p>
        <ArchiveOutputSummary :output="store.task.archiveOutput" />
      </section>
      <p v-if="store.task?.status === 'cancelled'" class="text-sm text-amber-600">转换已取消 · 原压缩包已保留</p>
      <p v-if="store.error" role="alert" class="text-sm text-red-500 break-words">{{ store.error }}</p>
      <p v-if="store.busy" class="text-sm text-primary">{{ store.phase }} · 以下为当前步骤进度</p>
      <div v-if="store.busy && store.task" class="rounded-xl border border-subtle p-3 space-y-2 text-sm">
        <span>{{ store.task.status === 'cancelling' ? '正在取消，等待清理完成' : `当前步骤：${formatProgressPercent(store.task.progress)}%` }}</span>
        <progress class="w-full" :value="store.task.progress" max="100" />
      </div>
      <details v-if="store.task" class="text-sm text-muted"><summary class="cursor-pointer">执行日志</summary><div class="max-h-48 overflow-y-auto space-y-1 mt-2"><p v-for="(log, index) in store.task.logs" :key="index" class="break-words">{{ log.message }}</p></div></details>
      <p v-if="store.task?.historySaveError" role="alert" class="text-sm text-red-500">{{ store.task.historySaveError }}</p>
      <fieldset :disabled="store.busy" class="space-y-4 disabled:opacity-70">
        <p v-if="store.contextIntent === 'reconfigure'" class="rounded-xl bg-primary/5 p-3 text-xs leading-relaxed text-muted">填写新包密码可以修改密码，留空则去除密码；填写分卷大小可以重新分卷，0 表示合并为一个文件。原压缩包会保留。</p>
        <label class="block text-sm">原压缩包<span v-if="store.contextSources.length > 1" class="ml-2 text-xs text-muted">已带入 {{ store.contextSources.length }} 项，可逐个切换配置</span>
          <div class="flex gap-2 mt-1">
            <ThemedSelect v-if="store.contextSources.length > 1" class="min-w-0 flex-1" :model-value="store.sourcePath" :options="store.contextSources.map(path => ({ value: path, label: path.split(/[\\/]/).pop() || path, detail: path }))" :disabled="store.busy" aria-label="选择要转换的压缩包" test-id="conversion-context-source" floating @update:model-value="store.chooseContextSource($event)" />
            <input v-else v-model="store.sourcePath" data-testid="conversion-source" class="conversion-input min-w-0 flex-1" placeholder="请选择原压缩包" />
            <button class="conversion-button" data-testid="conversion-choose-source" @click="chooseSource">选择文件</button>
          </div>
        </label>
        <label class="block text-sm">新包保存位置<div class="flex gap-2 mt-1"><input v-model="store.outputPath" data-testid="conversion-output" class="conversion-input min-w-0 flex-1" :placeholder="`新文件名.${store.format}`" /><button class="conversion-button" data-testid="conversion-choose-output" @click="chooseOutput">另存为</button></div></label>
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <label class="block text-sm">原包密码<input v-model="store.sourcePassword" type="password" autocomplete="off" data-testid="conversion-source-password" class="conversion-input w-full mt-1" placeholder="原包未加密可留空" /></label>
          <label class="block text-sm">目标格式<ThemedSelect v-model="store.format" :disabled="store.busy" :options="conversionFormats" test-id="conversion-format" aria-label="转换目标格式" floating /></label>
          <label class="block text-sm">新包密码<input v-model="store.password" :disabled="!store.supportsPassword || store.busy" type="password" autocomplete="new-password" data-testid="conversion-password" class="conversion-input w-full mt-1" placeholder="留空不加密" /></label>
          <label class="block text-sm">分卷大小（MiB）<input v-model.number="store.splitMiB" :disabled="!store.supportsPassword || store.busy" type="number" min="0" data-testid="conversion-split" class="conversion-input w-full mt-1" /><span class="text-xs text-muted">0 表示不分卷</span></label>
          <label class="block text-sm">压缩强度 {{ store.level }} / 9<input v-model.number="store.level" type="range" min="0" max="9" class="w-full" /></label>
          <template v-if="store.format === 'zip' && store.password">
            <label class="text-sm">ZIP 加密方式<ThemedSelect :model-value="store.zipEncryption" @update:model-value="store.zipEncryption = $event as 'aes256' | 'zipcrypto'" :disabled="store.busy" :options="[{ value: 'aes256', label: 'AES-256（推荐）' }, { value: 'zipcrypto', label: 'ZipCrypto（传统兼容，较弱）' }]" test-id="conversion-zip-encryption" floating /></label>
            <label class="text-sm">密码编码<ThemedSelect :model-value="store.zipPasswordEncoding" @update:model-value="store.zipPasswordEncoding = $event as 'utf8' | 'windows_local'" :disabled="store.busy" :options="[{ value: 'utf8', label: 'UTF-8' }, { value: 'windows_local', label: 'Windows 兼容' }]" test-id="conversion-zip-encoding" floating /></label>
          </template>
          <label v-if="store.format === '7z'" class="flex items-center gap-2 text-sm"><input v-model="store.encryptFilenames" type="checkbox" data-testid="conversion-hidden-names" />加密文件名</label>
        </div>
      </fieldset>
    </div>
    <template #footer>
      <button class="conversion-button" @click="store.visible = false">{{ store.busy ? '后台继续' : '关闭' }}</button>
      <button v-if="store.busy" class="conversion-button" data-testid="conversion-cancel" @click="store.cancel">取消转换</button>
      <button v-else class="conversion-button bg-primary text-white" data-testid="conversion-start" @click="store.start">{{ store.contextIntent === 'reconfigure' ? '生成新压缩包' : '开始转换' }}</button>
    </template>
  </Modal>
</template>
<style scoped>
.conversion-input { border: 1px solid var(--border-subtle, #cbd5e1); border-radius: .65rem; padding: .65rem; background: var(--bg-input, transparent); }
.conversion-button { flex-shrink: 0; border: 1px solid var(--border-subtle, #cbd5e1); border-radius: .65rem; padding: .6rem .9rem; font-size: .8rem; font-weight: 700; }
</style>
