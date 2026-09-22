import { ref } from 'vue'
import { useTaskStore } from '@/stores/task'

const createSession = () => ({ isRunning: ref(false), activeTaskId: null as string | null, stopRequested: false })
const sessions = new WeakMap<ReturnType<typeof useTaskStore>, Map<string, ReturnType<typeof createSession>>>()

// Native work outlives route components. Keep control per app and workload,
// including planning and history persistence, without persisting a running job.
export const useMediaBatchSession = (kind: 'video' | 'pdf') => {
  const store = useTaskStore()
  let appSessions = sessions.get(store)
  if (!appSessions) { appSessions = new Map(); sessions.set(store, appSessions) }
  let session = appSessions.get(kind)
  if (!session) { session = createSession(); appSessions.set(kind, session) }
  return session
}
