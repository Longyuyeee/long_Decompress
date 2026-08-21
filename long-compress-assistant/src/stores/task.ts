import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/tauri'
import type { ResourcePreflightReport } from '@/types/resourcePreflight'
import { createTaskHistoryRecord } from '@/types/taskHistory'
import { normalizeProgressPercent } from '@/utils/progress'

export type TaskStatus = 'pending' | 'preparing' | 'running' | 'compressing' | 'extracting' | 'finalizing' | 'cancelling' | 'completed' | 'failed' | 'cancelled'
export type LogSeverity = 'info' | 'warning' | 'error' | 'success'
export type TaskType = 'compression' | 'decompression'

export interface TaskLog {
  task_id: string
  timestamp: string
  message: string
  severity: LogSeverity
}

export interface ConflictInfo {
  taskId: string
  fileName: string
  sourcePath: string
  destPath: string
  sourceSize: number
  destSize: number
  sourceModified: number
  destModified: number
}

export interface Task {
  id: string
  name: string
  type: TaskType
  status: TaskStatus
  progress: number
  startTime?: Date
  endTime?: Date
  error?: string
  logs: TaskLog[]
  sourceFiles: string[]
  outputPath: string
  format?: string
  conflicts: ConflictInfo[]
  extractToSubfolder?: boolean
  recycleSourceAfterExtract?: boolean
  fileFilter?: string
  selectedEntries?: string[]
  // 增强字段 [FE-INT-001]
  stage?: 'Pre-checking' | 'Extracting' | 'Verifying' | 'Finalizing' | 'password-attempt'
  currentFile?: string
  currentPassword?: string
  passwordAttemptCurrent?: number
  passwordAttemptTotal?: number
  speed?: string
  processedBytes?: number
  totalBytes?: number
  etaSeconds?: number
  // 密码相关
  password?: string
  passwordRequired?: boolean
  compressionOptions?: {
    format?: string
    level: number
    password?: string
    split_size?: number | null
    create_solid_archive?: boolean
    preserve_paths?: boolean
    delete_after?: boolean
    verify_after?: boolean
    allow_insecure_password_cli?: boolean
  }
  resourcePreflight?: ResourcePreflightReport
}

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const activeTaskCount = computed(() => tasks.value.filter(t => !['completed', 'failed', 'cancelled'].includes(t.status)).length)
  const tasksFor = (type: TaskType) => tasks.value.filter(task => task.type === type)
  let listenerInitialization: Promise<void> | null = null

  // 初始化监听器
  const initListeners = () => {
    if (listenerInitialization) return listenerInitialization

    listenerInitialization = (async () => {
    await listen<TaskLog>('task-log', (event) => {
      const { task_id, message, severity, timestamp } = event.payload
      const task = tasks.value.find(t => t.id === task_id)
      if (task) {
        task.logs.push({
          task_id,
          message,
          severity: severity.toLowerCase() as LogSeverity,
          timestamp
        })
      }
    })

    await listen<{ 
      task_id: string, 
      progress: number,
      stage?: string,
      current_file?: string,
      current_password?: string,
      speed?: string,
      processed_bytes?: number,
      total_bytes?: number,
      eta_seconds?: number,
      password_attempt_current?: number,
      password_attempt_total?: number,
    }>('task-progress', (event) => {
      const {
        task_id, progress, stage, current_file, current_password, speed,
        processed_bytes, total_bytes, eta_seconds,
        password_attempt_current, password_attempt_total,
      } = event.payload
      const task = tasks.value.find(t => t.id === task_id)
      if (task) {
        task.progress = normalizeProgressPercent(progress)
        task.stage = stage !== undefined
          ? stage as any
          : task.status === 'extracting'
            ? 'Extracting'
            : task.stage
        task.currentFile = current_file
        task.currentPassword = current_password
        task.passwordAttemptCurrent = password_attempt_current
        task.passwordAttemptTotal = password_attempt_total
        if (speed !== undefined) task.speed = speed
        // Stage-only completion events use zero byte fields. Do not let those
        // erase real transfer totals already emitted by the archive engine.
        if (processed_bytes !== undefined && (processed_bytes > 0 || !task.processedBytes)) {
          task.processedBytes = processed_bytes
        }
        if (total_bytes !== undefined && (total_bytes > 0 || !task.totalBytes)) {
          task.totalBytes = total_bytes
        }
        if (eta_seconds !== undefined) task.etaSeconds = eta_seconds

        // Progress reaching 100% only means the engine finished transferring data.
        // Final rename, integrity checks and optional cleanup may still fail, so the
        // command owner is the only place allowed to mark a task completed.
      }
    })

    // 监听密码需求事件 - 后端检测到加密文件但密码为空时发送
    await listen<{
      task_id: string
      file_path: string
      file_name: string
      format: string
    }>('password-required', (event) => {
      const { task_id, file_path, file_name, format } = event.payload
      const task = tasks.value.find(t => t.id === task_id)
      if (task) {
        task.passwordRequired = true
        task.status = 'pending'
        task.logs.push({
          task_id,
          message: `需要密码: ${file_name} (${format})`,
          severity: 'warning',
          timestamp: new Date().toISOString()
        })
      }
    })

    // 监听冲突事件
    await listen<ConflictInfo>('file-conflict', (event) => {
      const conflict = event.payload
      const task = tasks.value.find(t => t.id === conflict.taskId)
      if (task) {
        task.conflicts.push(conflict)
      }
    })
    })().catch((error) => {
      listenerInitialization = null
      throw error
    })

    return listenerInitialization
  }

  const addTask = (task: Omit<Task, 'logs' | 'progress' | 'status' | 'conflicts'>) => {
    const newTask: Task = {
      ...task,
      status: 'pending',
      progress: 0,
      logs: [],
      conflicts: []
    }
    tasks.value.push(newTask)
    return newTask.id
  }

  const updateTaskStatus = (taskId: string, status: TaskStatus) => {
    const task = tasks.value.find(t => t.id === taskId)
    if (task) {
      task.status = status
      if (['preparing', 'running', 'compressing', 'extracting', 'finalizing'].includes(status) && !task.startTime) {
        task.startTime = new Date()
      }
      if (['completed', 'failed', 'cancelled'].includes(status)) {
        task.endTime = new Date()
        task.etaSeconds = undefined
        const historyRecord = createTaskHistoryRecord(task)
        void invoke('save_task_history', { record: historyRecord }).catch((error) => {
          console.warn('Failed to persist task history:', error)
        })
      }
    }
  }

  const removeTask = (taskId: string) => {
    const index = tasks.value.findIndex(t => t.id === taskId)
    if (index !== -1) {
      tasks.value.splice(index, 1)
    }
  }

  const cancelTask = async (taskId: string) => {
    const task = tasks.value.find(item => item.id === taskId)
    if (!task || ['completed', 'failed', 'cancelled', 'cancelling'].includes(task.status)) {
      return false
    }
    const previousStatus = task.status
    updateTaskStatus(taskId, 'cancelling')
    try {
      await invoke('cancel_compression', { taskId })
      updateTaskStatus(taskId, 'cancelled')
      return true
    } catch (e) {
      console.error('Failed to cancel task:', e)
      updateTaskStatus(taskId, previousStatus)
      // 将错误信息记录到任务的日志中
      if (task) {
        task.logs.push({
          task_id: taskId,
          message: `Cancel failed: ${e}`,
          severity: 'error',
          timestamp: new Date().toISOString()
        })
      }
      return false
    }
  }

  const fetchTasks = async () => {
    // 这是一个占位符，如果后端支持获取历史任务，可以在此实现
    // console.log('Fetching tasks...')
  }

  const clearFinishedTasks = (type?: TaskType) => {
    tasks.value = tasks.value.filter(task => {
      const isFinished = ['completed', 'failed', 'cancelled'].includes(task.status)
      return !isFinished || (type !== undefined && task.type !== type)
    })
  }

  return {
    tasks,
    activeTaskCount,
    tasksFor,
    initListeners,
    addTask,
    updateTaskStatus,
    removeTask,
    clearFinishedTasks,
    cancelTask,
    fetchTasks
  }
})
