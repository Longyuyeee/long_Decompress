<script setup lang="ts">
import { open } from '@tauri-apps/api/dialog'
import { ref } from 'vue'
import type { ConversionSettings, ConversionPatch } from '@/types/archiveConversion'
import { formatFileSize } from '@/utils'
const props = defineProps<{ settings: ConversionSettings; disabled?: boolean; sourcePasswordRequired?: boolean; baseBytes?: number }>()
const emit = defineEmits<{ update: [patch: ConversionPatch] }>()
const selectionError = ref('')
const name = (path: string) => path.split(/[\\/]/).pop() || path
const destination = (path: string) => [props.settings.archivePrefix?.replace(/\\/g,'/').replace(/^\/+|\/+$/g,''), name(path)].filter(Boolean).join('/')
async function choose(directory = false) {
  selectionError.value = ''
  try {
  const queued = import.meta.env.VITE_DESKTOP_E2E === '1' ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
  const result = queued === undefined ? await open({ directory, multiple: true, title: directory ? '选择本次添加/更新的文件夹' : '选择本次添加/更新的文件' }) : queued
  const paths = typeof result === 'string' ? [result] : Array.isArray(result) ? result : []
  if (paths.length) emit('update', { updateSources: [...new Set([...(props.settings.updateSources || []), ...paths])] })
  } catch (error) { selectionError.value = '选择内容失败：' + String(error) }
}
</script>
<template>
  <fieldset :disabled="disabled" class="min-w-0 space-y-4 disabled:opacity-60" data-testid="zpaq-update-fields">
    <p v-if="selectionError" role="alert" class="break-words text-xs text-red-500">{{ selectionError }}</p>
    <div class="rounded-xl border border-primary/20 bg-primary/5 px-3 py-3 text-xs leading-relaxed text-muted">
      <p class="font-semibold text-content">另存更新包，保留原备份与全部历史快照</p>
      <p class="mt-1">先复制完整原包<span v-if="baseBytes">（{{ formatFileSize(baseBytes) }}）</span>，再比较内容，只压缩新增或变化数据。保存位置需要容纳副本和新增内容；重新分卷还需要一份切卷空间。</p>
      <p class="mt-1">更新以最新快照为基础。同路径条目会更新，其余旧内容保留；本次不做删除同步或改密。</p>
    </div>
    <div class="min-w-0 space-y-2">
      <div class="flex flex-wrap items-center gap-2 text-sm"><span class="mr-auto font-semibold">本次添加 / 更新的内容</span><button type="button" class="conversion-button" data-testid="zpaq-update-add-files" @click="choose()"><i class="pi pi-file"></i>添加文件</button><button type="button" class="conversion-button" data-testid="zpaq-update-add-folder" @click="choose(true)"><i class="pi pi-folder"></i>添加文件夹</button></div>
      <div v-if="!settings.updateSources?.length" class="rounded-xl border border-dashed border-primary/25 px-3 py-5 text-center text-xs text-muted">选择本次要写入备份的内容，下面会显示归档内位置。</div>
      <ol v-else class="max-h-52 space-y-2 overflow-y-auto rounded-xl border border-subtle bg-input/20 p-3" data-testid="zpaq-update-sources">
        <li v-for="path in settings.updateSources" :key="path" class="flex min-w-0 items-start gap-2 text-xs">
          <div class="min-w-0 flex-1"><p class="break-all font-semibold text-content">{{ name(path) }} <span class="font-normal text-primary">→ {{ destination(path) }}</span></p><p class="mt-1 break-all text-muted">{{ path }}</p></div>
          <button type="button" class="shrink-0 p-1 text-muted hover:text-red-500" :aria-label="'移除更新来源 ' + name(path)" @click="emit('update', { updateSources: settings.updateSources!.filter(p => p !== path) })"><i class="pi pi-times"></i></button>
        </li>
      </ol>
      <p class="text-xs leading-relaxed text-muted">所选文件夹保留名称及内部层级；原备份分卷和本次临时产物会排除。请核对位置，同名路径才会更新。</p>
    </div>
    <div class="grid min-w-0 grid-cols-1 gap-4 sm:grid-cols-2">
      <label class="min-w-0 space-y-1.5 text-xs">归档内位置<input :value="settings.archivePrefix || ''" data-testid="zpaq-update-prefix" class="conversion-input w-full" placeholder="留空表示备份根目录，例如 资料/项目" @input="emit('update', { archivePrefix: ($event.target as HTMLInputElement).value })" /><span class="block text-muted">例如选择“说明.txt”并填写“资料”，将更新“资料/说明.txt”。</span></label>
      <div class="min-w-0 space-y-1.5 text-xs"><label class="block">原备份密码<input :value="settings.sourcePassword" data-testid="zpaq-update-password" class="conversion-input mt-1.5 w-full" type="password" autocomplete="off" placeholder="原包未加密可留空；新包沿用原密码" @input="emit('update', { sourcePassword: ($event.target as HTMLInputElement).value })" /></label><button v-if="sourcePasswordRequired" type="button" class="text-primary hover:underline" @click="emit('update', { sourcePassword: '' })">原备份无需密码</button></div>
      <label class="min-w-0 space-y-1.5 text-xs">压缩强度 · {{ settings.level }} / 9<input type="range" min="0" max="9" :value="settings.level" class="block w-full accent-primary" data-testid="zpaq-update-level" @input="emit('update', { level: Number(($event.target as HTMLInputElement).value) })" /><span class="block text-muted">只影响本次新增数据，旧数据与快照保留。</span></label>
      <label class="min-w-0 space-y-1.5 text-xs">新包分卷大小（MiB）<input :value="settings.splitMiB" type="number" min="0" step="any" class="conversion-input w-full" data-testid="zpaq-update-split" @change="emit('update', { splitMiB: Number(($event.target as HTMLInputElement).value) })" /><span class="block text-muted">0 为单包；其他数值生成新的数字卷，原卷组保留。</span></label>
    </div>
  </fieldset>
</template>
