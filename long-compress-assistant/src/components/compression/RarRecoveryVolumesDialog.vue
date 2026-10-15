<script setup lang="ts">
import { onUnmounted, ref, watch } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { parentDirectory } from '@/utils/resultLocation'
const props = defineProps<{ visible: boolean; path: string; password?: string }>()
const emit = defineEmits<{ 'update:visible': [value: boolean] }>()
const commands = useTauriCommands()
const directory = ref(''), password = ref(''), count = ref('1'), error = ref(''), operationId = ref('')
const busy = ref(false), cancelling = ref(false)
const result = ref<{outputFiles: string[]; totalBytes: number; dataVolumes: number} | null>(null)
watch(() => props.visible, visible => {
  if (visible) { directory.value = parentDirectory(props.path); password.value = props.password || ''; result.value = null; error.value = ''; cancelling.value = false }
  else password.value = ''
}, {immediate:true})
async function chooseDirectory() {
  const queued = import.meta.env.VITE_DESKTOP_E2E === '1' ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
  const selected = queued === undefined ? await open({directory:true,multiple:false,title:'选择恢复卷保存目录'}) : queued
  if (typeof selected === 'string') directory.value = selected
}
async function create() {
  if (busy.value || !directory.value.trim()) return
  busy.value = true; error.value = ''; result.value = null; cancelling.value = false
  operationId.value = `recovery-volumes-${Date.now()}-${Math.random().toString(36).slice(2)}`
  try { result.value = await commands.createRarRecoveryVolumes(operationId.value, props.path, directory.value.trim(), Number(count.value), password.value) }
  catch (failure) { if (!String(failure).toLowerCase().includes('cancel')) error.value = String(failure) }
  finally { busy.value = false; operationId.value = ''; cancelling.value = false }
}
async function cancel() {
  if (!operationId.value || cancelling.value) return
  cancelling.value = true
  try { await commands.cancelZipRepair(operationId.value) } catch { /* completed concurrently */ }
}
onUnmounted(() => { void cancel(); password.value = '' })
</script>
<template>
  <Modal :visible="visible" title="创建 RAR 恢复卷" icon="pi pi-shield" size="md" :show-close-button="!busy" :close-on-backdrop="!busy" :close-on-escape="!busy" @update:visible="emit('update:visible', $event)">
    <div class="space-y-4" data-testid="create-recovery-volumes-dialog">
      <div class="rounded-xl border border-subtle bg-input/40 p-3 text-xs leading-relaxed text-muted">
        <p class="break-all font-semibold text-content">{{ path.split(/[\\/]/).pop() }}</p>
        <p class="mt-1">为完整分包补充 .rev，已有数据卷保持不变。每个恢复卷大小接近一个数据卷，请与原分包整组保留。</p>
      </div>
      <label class="block space-y-2 text-xs text-muted">恢复卷数量<ThemedSelect v-model="count" :disabled="busy" :options="[1,2,3,5,10].map(value => ({value:String(value),label:`${value} 个恢复卷`}))" test-id="create-recovery-count" floating /></label>
      <label class="block space-y-2 text-xs text-muted">原包密码<input v-model="password" :disabled="busy" type="password" autocomplete="off" placeholder="未加密可留空" class="w-full rounded-xl border border-subtle bg-input px-3 py-2.5 text-sm text-content" data-testid="create-recovery-password" /></label>
      <label class="block space-y-2 text-xs text-muted">保存目录<div class="flex min-w-0 gap-2"><input v-model="directory" :disabled="busy" class="min-w-0 flex-1 rounded-xl border border-subtle bg-input px-3 py-2.5 text-sm text-content" data-testid="create-recovery-directory" /><button class="shrink-0 rounded-xl border border-subtle px-3 text-sm" :disabled="busy" @click="chooseDirectory">选择</button></div></label>
      <p v-if="busy" class="text-xs text-primary"><i class="pi pi-spin pi-spinner mr-2"></i>{{ cancelling ? '正在取消…' : '正在读取分包并生成恢复卷，大分包需要一些时间…' }}</p>
      <p v-if="error" role="alert" class="break-words rounded-xl bg-red-500/10 p-3 text-xs leading-relaxed text-red-500">{{ error }}</p>
      <div v-if="result" class="rounded-xl border border-green-500/30 bg-green-500/5 p-3 text-sm" data-testid="created-recovery-volumes">
        <p class="font-bold text-green-600">已为 {{ result.dataVolumes }} 个数据卷生成 {{ result.outputFiles.length }} 个恢复卷</p>
        <ul class="mt-2 space-y-1 text-xs text-muted"><li v-for="file in result.outputFiles" :key="file" class="break-all">{{ file }}</li></ul>
        <button class="mt-3 font-semibold text-primary" @click="commands.invoke('open_in_explorer', {path:result.outputFiles[0]})"><i class="pi pi-folder-open mr-2"></i>定位恢复卷</button>
      </div>
    </div>
    <template #footer>
      <button v-if="busy" class="rounded-xl border border-subtle px-4 py-2 text-sm" :disabled="cancelling" @click="cancel">{{ cancelling ? '正在取消…' : '取消生成' }}</button>
      <template v-else><button class="rounded-xl border border-subtle px-4 py-2 text-sm" @click="emit('update:visible', false)">{{ result ? '完成' : '关闭' }}</button><button v-if="!result" class="rounded-xl bg-primary px-4 py-2 text-sm font-bold text-white disabled:opacity-40" :disabled="!directory.trim()" data-testid="confirm-create-recovery" @click="create">生成恢复卷</button></template>
    </template>
  </Modal>
</template>
