<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { formatProgressPercent } from '@/utils/progress'

const props = withDefaults(defineProps<{ value?: number, suffix?: string, decimals?: 0 | 2 }>(), {
  value: 0,
  suffix: '%',
  decimals: 2,
})

const clampPercent = (value?: number) => typeof value === 'number' && Number.isFinite(value)
  ? Math.min(100, Math.max(0, value))
  : 0

const displayed = ref(clampPercent(props.value))
let frame = 0

const animateTo = (rawTarget: number) => {
  const target = clampPercent(rawTarget)
  const start = displayed.value
  cancelAnimationFrame(frame)
  if (target <= start || window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
    displayed.value = target
    return
  }
  const startedAt = performance.now()
  const duration = Math.min(900, Math.max(360, (target - start) * 22))
  const tick = (now: number) => {
    const elapsed = Math.min(1, (now - startedAt) / duration)
    displayed.value = start + (target - start) * (1 - Math.pow(1 - elapsed, 3))
    if (elapsed < 1) frame = requestAnimationFrame(tick)
  }
  frame = requestAnimationFrame(tick)
}

watch(() => props.value, animateTo)
onBeforeUnmount(() => cancelAnimationFrame(frame))

const formatDisplay = (value: number) => props.decimals === 0
  ? Math.round(value).toFixed(0)
  : formatProgressPercent(value)
</script>

<template>
  <span class="tabular-nums whitespace-nowrap">{{ formatDisplay(displayed) }}{{ suffix }}</span>
</template>
