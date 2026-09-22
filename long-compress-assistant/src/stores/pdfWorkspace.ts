import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { PdfInputAnalysisReport, PdfOptimizationMode } from '@/types/pdf'

export interface PdfWorkspaceItem {
  id: string
  path: string
  name: string
  status: 'analyzing' | 'password-required' | 'ready' | 'blocked' | 'failed'
  report: PdfInputAnalysisReport | null
  mode: PdfOptimizationMode
  password: string
  riskConfirmed: boolean
  frozen: boolean
  allowLargerOutput: boolean
  taskId: string | null
  error: string
}

// In-memory draft only: switching tabs must not discard the task card.
// No password or running task is serialized across application restarts.
export const usePdfWorkspaceStore = defineStore('pdf-workspace', () => {
  const items = ref<PdfWorkspaceItem[]>([])
  const outputDirectory = ref('')
  return { items, outputDirectory }
})
