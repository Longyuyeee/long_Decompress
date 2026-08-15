import { describe, expect, it } from 'vitest'
import { runArchiveTasks } from '../taskConcurrency'

describe('runArchiveTasks', () => {
  it('honors the configured concurrency limit', async () => {
    let active = 0
    let peak = 0

    await runArchiveTasks([1, 2, 3, 4, 5], 2, async () => {
      active++
      peak = Math.max(peak, active)
      await new Promise(resolve => setTimeout(resolve, 5))
      active--
    })

    expect(peak).toBe(2)
  })

  it('serializes tasks that share an output resource', async () => {
    const activeKeys = new Set<string>()
    const overlaps: string[] = []
    let peak = 0

    await runArchiveTasks(
      [
        { id: 1, output: 'same' },
        { id: 2, output: 'other' },
        { id: 3, output: 'same' },
      ],
      3,
      async item => {
        if (activeKeys.has(item.output)) overlaps.push(item.output)
        activeKeys.add(item.output)
        peak = Math.max(peak, activeKeys.size)
        await new Promise(resolve => setTimeout(resolve, 5))
        activeKeys.delete(item.output)
      },
      item => item.output,
    )

    expect(overlaps).toEqual([])
    expect(peak).toBe(2)
  })
})
