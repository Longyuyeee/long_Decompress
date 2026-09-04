import { describe, expect, it } from 'vitest'
import { sortTasksByName } from '../taskOrdering'

describe('sortTasksByName', () => {
  it('uses the same stable natural order for display and execution', () => {
    const tasks = [
      { id: '5', name: '1 (3)-3.rar' },
      { id: '2', name: '1 (1)-1.rar' },
      { id: '4', name: '1 (2)-2.rar' },
      { id: '1', name: '1 (1)-1 (2).rar' },
      { id: '3', name: '1 (2)-2 (2).rar' },
    ]

    expect(sortTasksByName(tasks).map(task => task.id)).toEqual(['1', '2', '3', '4', '5'])
  })

  it('does not mutate the task store order', () => {
    const tasks = [{ id: '2', name: 'b.rar' }, { id: '1', name: 'a.rar' }]
    sortTasksByName(tasks)
    expect(tasks.map(task => task.id)).toEqual(['2', '1'])
  })
})
