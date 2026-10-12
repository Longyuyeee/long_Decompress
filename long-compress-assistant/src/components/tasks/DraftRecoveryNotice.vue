<script setup lang="ts">
import { ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
const props = defineProps<{ sources: Array<{ path: string; issue?: string; isDirectory?: boolean }>; needsPassword?: boolean }>()
const emit = defineEmits<{ replace: [index: number, path: string, metadata: { name: string; size: number; is_dir: boolean }]; password: [value: string] }>()
const password = ref(''), error = ref(''), busy = ref(false)
async function replace(index: number) {
  busy.value = true; error.value = ''
  try {
    const path = await open({ multiple: false, directory: props.sources[index].isDirectory, title: '重新选择来源' })
    if (typeof path !== 'string') return
    const metadata = await invoke<{ name: string; size: number; is_dir: boolean }>('get_file_info', { path })
    emit('replace', index, path, metadata)
  } catch { error.value = '无法读取所选来源，请重新选择。' }
  finally { busy.value = false }
}
</script>
<template>
  <div v-if="sources.some(source => source.issue) || needsPassword" class="space-y-2 rounded-xl border border-amber-500/25 bg-amber-500/5 p-3 text-xs" data-testid="draft-recovery-notice">
    <div v-for="(source, index) in sources" :key="index">
      <div v-if="source.issue" class="flex min-w-0 flex-wrap items-center gap-2"><span class="min-w-0 flex-1 break-words text-amber-700 dark:text-amber-400">{{ source.issue }} · {{ source.path }}</span><button type="button" class="shrink-0 rounded-lg border border-subtle px-2.5 py-1.5 text-primary" :disabled="busy" @click="replace(index)">重新选择来源</button></div>
    </div>
    <div v-if="needsPassword" class="space-y-2">
      <p class="text-muted">此任务使用过密码，恢复后请重新填写。</p>
      <div class="flex min-w-0 flex-wrap gap-2"><input v-model="password" type="password" autocomplete="off" class="min-w-0 flex-1 rounded-lg border border-subtle bg-input px-2.5 py-2" placeholder="输入密码；留空表示不再使用" /><button type="button" class="rounded-lg bg-primary px-3 py-2 text-white" @click="emit('password', password); password = ''">确认密码设置</button></div>
    </div>
    <p v-if="error" role="alert" class="text-red-500">{{ error }}</p>
  </div>
</template>
