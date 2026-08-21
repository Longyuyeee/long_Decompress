export const normalizeProgressPercent = (fraction: number) => {
  if (!Number.isFinite(fraction) || fraction <= 0) return 0
  if (fraction >= 1) return 100
  return Math.max(0.01, Math.round(fraction * 10_000) / 100)
}

export const formatProgressPercent = (percent?: number) => {
  const value = typeof percent === 'number' && Number.isFinite(percent)
    ? Math.min(100, Math.max(0, percent))
    : 0
  if (value === 0 || value === 100) return value.toFixed(0)
  return value.toFixed(2)
}
