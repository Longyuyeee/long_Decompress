type Release = () => void

const createLimiter = (requestedLimit: number) => {
  const limit = Math.max(1, Math.floor(Number.isFinite(requestedLimit) ? requestedLimit : 1))
  let active = 0
  const waiting: Array<(release: Release) => void> = []

  const release = () => {
    active = Math.max(0, active - 1)
    const next = waiting.shift()
    if (next) {
      active++
      next(release)
    }
  }

  return () => {
    if (active < limit) {
      active++
      return Promise.resolve(release)
    }
    return new Promise<Release>(resolve => waiting.push(resolve))
  }
}

/**
 * Runs independent archive tasks with a bounded global concurrency. Tasks that
 * share a resource key are serialized, which prevents two extractions from
 * committing into the same output directory at the same time.
 */
export const runArchiveTasks = async <T>(
  items: readonly T[],
  requestedLimit: number,
  worker: (item: T, index: number) => Promise<void>,
  resourceKey?: (item: T, index: number) => string | undefined,
) => {
  const acquire = createLimiter(requestedLimit)
  const resourceTails = new Map<string, Promise<void>>()

  await Promise.all(items.map(async (item, index) => {
    const key = resourceKey?.(item, index)
    const previous = key ? resourceTails.get(key) : undefined
    let releaseResource: (() => void) | undefined
    const current = key
      ? new Promise<void>(resolve => { releaseResource = resolve })
      : undefined

    if (key && current) resourceTails.set(key, current)
    if (previous) await previous.catch(() => {})

    const releaseSlot = await acquire()
    try {
      await worker(item, index)
    } finally {
      releaseSlot()
      releaseResource?.()
      if (key && current && resourceTails.get(key) === current) {
        resourceTails.delete(key)
      }
    }
  }))
}
