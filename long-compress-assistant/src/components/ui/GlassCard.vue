<template>
  <div :class="[
    'glass-card',
    'rounded-xl',
    'p-6',
    'transition-all',
    'duration-300',
    'backdrop-blur-md',
    'border',
    'shadow-lg',
    hoverable ? 'hover:bg-white/15 dark:hover:bg-black/15 hover:shadow-xl' : '',
    compact ? 'p-4' : '',
    className
  ]" :style="style">
    <slot />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

export interface Props {
  hoverable?: boolean
  compact?: boolean
  className?: string
  backgroundColor?: string
  borderColor?: string
  blur?: string
}

const props = withDefaults(defineProps<Props>(), {
  hoverable: true,
  compact: false,
  className: '',
  backgroundColor: '',
  borderColor: '',
  blur: 'md'
})

const style = computed(() => {
  const styles: Record<string, string> = {}

  if (props.backgroundColor) {
    styles.backgroundColor = props.backgroundColor
  } else {
    styles.backgroundColor = 'rgba(255, 255, 255, 0.1)'
  }

  if (props.borderColor) {
    styles.borderColor = props.borderColor
  } else {
    styles.borderColor = 'rgba(255, 255, 255, 0.2)'
  }

  if (props.blur) {
    styles.backdropFilter = `blur(${props.blur === 'md' ? '12px' : props.blur})`
  }

  return styles
})
</script>

<style scoped>
.glass-card {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.05) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.2);
  box-shadow:
    0 8px 32px 0 rgba(31, 38, 135, 0.37),
    inset 0 1px 0 0 rgba(255, 255, 255, 0.1);
}

.dark .glass-card {
  background: linear-gradient(
    135deg,
    rgba(0, 0, 0, 0.1) 0%,
    rgba(0, 0, 0, 0.05) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.1);
}
</style>

