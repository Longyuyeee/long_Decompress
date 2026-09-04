export interface NamedTask {
  id: string
  name: string
}

const taskNameCollator = new Intl.Collator('zh-CN', {
  numeric: true,
  sensitivity: 'base',
})

export const compareTasksByName = (left: NamedTask, right: NamedTask) =>
  taskNameCollator.compare(left.name || '', right.name || '') || left.id.localeCompare(right.id)

export const sortTasksByName = <T extends NamedTask>(tasks: readonly T[]) =>
  [...tasks].sort(compareTasksByName)
