<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  text: string
  delay?: number
}>(), {
  delay: 220,
})

const anchor = ref<HTMLElement | null>(null)
const tooltip = ref<HTMLElement | null>(null)
const visible = ref(false)
const placement = ref<'above' | 'below'>('below')
const position = ref({ left: 0, top: 0, maxWidth: 420 })
const tooltipId = `overflow-tooltip-${Math.random().toString(36).slice(2, 10)}`
let showTimer: ReturnType<typeof setTimeout> | undefined

const isOverflowing = () => {
  const element = anchor.value
  return !!element && element.scrollWidth > element.clientWidth + 1
}

const updatePosition = () => {
  const element = anchor.value
  if (!element) return

  const rect = element.getBoundingClientRect()
  const maxWidth = Math.min(420, Math.max(180, window.innerWidth - 24))
  const renderedWidth = tooltip.value?.offsetWidth || maxWidth
  const halfWidth = Math.min(renderedWidth, maxWidth) / 2
  const left = Math.min(
    window.innerWidth - 12 - halfWidth,
    Math.max(12 + halfWidth, rect.left + rect.width / 2),
  )
  const showAbove = rect.top >= 88

  placement.value = showAbove ? 'above' : 'below'
  position.value = {
    left,
    top: showAbove ? rect.top - 10 : rect.bottom + 10,
    maxWidth,
  }
}

const show = () => {
  clearTimeout(showTimer)
  if (!props.text || !isOverflowing()) return

  showTimer = setTimeout(async () => {
    updatePosition()
    visible.value = true
    await nextTick()
    updatePosition()
  }, props.delay)
}

const hide = () => {
  clearTimeout(showTimer)
  visible.value = false
}

onBeforeUnmount(() => clearTimeout(showTimer))
</script>

<template>
  <span
    ref="anchor"
    v-bind="$attrs"
    class="overflow-tooltip-anchor"
    tabindex="0"
    :aria-describedby="visible ? tooltipId : undefined"
    @mouseenter="show"
    @mouseleave="hide"
    @focus="show"
    @blur="hide"
  >
    <slot>{{ text }}</slot>
  </span>

  <Teleport to="body">
    <Transition name="overflow-tooltip">
      <div
        v-if="visible"
        ref="tooltip"
        :id="tooltipId"
        role="tooltip"
        class="overflow-tooltip-popover"
        :class="`is-${placement}`"
        :style="{
          left: `${position.left}px`,
          top: `${position.top}px`,
          maxWidth: `${position.maxWidth}px`,
        }"
      >
        <i class="pi pi-file overflow-tooltip-icon" aria-hidden="true"></i>
        <span>{{ text }}</span>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.overflow-tooltip-anchor {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  outline: none;
}

.overflow-tooltip-anchor:focus-visible {
  border-radius: 0.25rem;
  outline: 2px solid color-mix(in srgb, var(--dynamic-accent) 72%, transparent);
  outline-offset: 3px;
}

.overflow-tooltip-popover {
  position: fixed;
  z-index: 1000;
  display: flex;
  align-items: flex-start;
  gap: 0.55rem;
  width: max-content;
  padding: 0.65rem 0.8rem;
  border: 1px solid color-mix(in srgb, var(--dynamic-accent) 34%, var(--border-subtle));
  border-radius: 0.75rem;
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--dynamic-accent) 9%, transparent), transparent 52%),
    color-mix(in srgb, var(--bg-card) 96%, black);
  box-shadow:
    0 16px 40px -18px rgba(0, 0, 0, 0.7),
    0 0 0 1px color-mix(in srgb, white 4%, transparent) inset;
  color: var(--text-content);
  font-size: 0.75rem;
  font-weight: 650;
  line-height: 1.45;
  overflow-wrap: anywhere;
  pointer-events: none;
  backdrop-filter: blur(16px);
}

.overflow-tooltip-popover.is-above {
  transform: translate(-50%, -100%);
  transform-origin: bottom center;
}

.overflow-tooltip-popover.is-below {
  transform: translateX(-50%);
  transform-origin: top center;
}

.overflow-tooltip-popover::after {
  content: '';
  position: absolute;
  left: 50%;
  width: 0.55rem;
  height: 0.55rem;
  border-right: 1px solid color-mix(in srgb, var(--dynamic-accent) 34%, var(--border-subtle));
  border-bottom: 1px solid color-mix(in srgb, var(--dynamic-accent) 34%, var(--border-subtle));
  background: color-mix(in srgb, var(--bg-card) 96%, black);
}

.overflow-tooltip-popover.is-above::after {
  bottom: -0.32rem;
  transform: translateX(-50%) rotate(45deg);
}

.overflow-tooltip-popover.is-below::after {
  top: -0.32rem;
  transform: translateX(-50%) rotate(225deg);
}

.overflow-tooltip-icon {
  margin-top: 0.12rem;
  flex: 0 0 auto;
  color: var(--dynamic-accent);
  font-size: 0.7rem;
}

.overflow-tooltip-enter-active,
.overflow-tooltip-leave-active {
  transition: opacity 150ms ease, filter 150ms ease;
}

.overflow-tooltip-enter-from,
.overflow-tooltip-leave-to {
  opacity: 0;
  filter: blur(3px);
}
</style>
