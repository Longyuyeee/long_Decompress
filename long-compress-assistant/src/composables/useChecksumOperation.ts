import { onUnmounted, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTauriCommands } from './useTauriCommands'
import type { ChecksumProgress } from '@/types/checksum'

/** Listen before invoking; discard late events and stop native I/O on leaving the workspace. */
export function useChecksumOperation() {
  const commands = useTauriCommands()
  const progress = ref<ChecksumProgress | null>(null), stopping = ref(false)
  let id = '', unlisten: UnlistenFn | undefined
  async function stop() {
    if (!id || stopping.value) return
    stopping.value = true
    try { await commands.invoke('cancel_checksum_operation', { operationId: id }) }
    catch { stopping.value = false }
  }
  async function run<T>(command: string, args: Record<string, unknown>, observe?: (value: ChecksumProgress) => void): Promise<T> {
    const operationId = crypto.randomUUID()
    id = operationId; progress.value = null; stopping.value = false
    try {
      const cleanup = await listen<ChecksumProgress>('checksum-progress', event => {
        if (id !== operationId || event.payload.operationId !== operationId) return
        progress.value = event.payload; observe?.(event.payload)
      })
      if (id !== operationId) { cleanup(); throw new Error('校验工作区已关闭') }
      unlisten = cleanup
      return await commands.invoke<T>(command, { ...args, operationId })
    } finally {
      if (id === operationId) { id = ''; unlisten?.(); unlisten = undefined; stopping.value = false }
    }
  }
  onUnmounted(() => { void stop(); id = ''; unlisten?.(); unlisten = undefined })
  return { progress, stopping, run, stop }
}
