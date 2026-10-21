export const taskScopes = [
  { id: 'all', label: '全部', icon: 'pi-list' },
  { id: 'pending', label: '待处理', icon: 'pi-clock' },
  { id: 'active', label: '进行中', icon: 'pi-spinner' },
  { id: 'completed', label: '成功结果', icon: 'pi-check-circle' },
  { id: 'attention', label: '失败/取消', icon: 'pi-exclamation-circle' },
] as const
export type TaskScope = typeof taskScopes[number]['id']
export function taskScope(status?: string): Exclude<TaskScope, 'all'> {
  if (!status || status === 'pending') return 'pending'
  if (status === 'completed') return 'completed'
  if (status === 'failed' || status === 'cancelled') return 'attention'
  return 'active'
}
export function matchesTaskScope(scope: TaskScope, status?: string) { return scope === 'all' || scope === taskScope(status) }
export function taskScopeCounts(statuses: (string | undefined)[]) {
  const counts = { all: statuses.length, pending: 0, active: 0, completed: 0, attention: 0 }
  for (const status of statuses) counts[taskScope(status)]++
  return counts
}
