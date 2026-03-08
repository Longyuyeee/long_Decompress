<template>
  <div :class="['progress-bar-container', className]">
    <!-- 标签 -->
    <div v-if="showLabel" class="flex justify-between mb-2">
      <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
        <slot name="label">{{ label }}</slot>
      </span>
      <span class="text-sm font-medium text-gray-900 dark:text-white">
        {{ formattedValue }}
      </span>
    </div>

    <!-- 进度条 -->
    <div class="relative">
      <!-- 背景轨道 -->
      <div
        :class="[
          'progress-bar-track',
          'w-full',
          'rounded-full',
          'overflow-hidden',
          size === 'sm' ? 'h-1.5' : '',
          size === 'md' ? 'h-2' : '',
          size === 'lg' ? 'h-3' : '',
          size === 'xl' ? 'h-4' : '',
          animated ? 'animate-pulse-slow' : ''
        ]"
        :style="{
          backgroundColor: trackColor
        }"
      ></div>

      <!-- 进度填充 -->
      <div
        :class="[
          'progress-bar-fill',
          'absolute',
          'top-0',
          'left-0',
          'h-full',
          'rounded-full',
          'transition-all',
          'duration-300',
          'ease-out',
          indeterminate ? 'indeterminate' : '',
          striped ? 'striped' : '',
          animated ? 'animate-pulse' : ''
        ]"
        :style="{
          width: indeterminate ? '100%' : `${percentage}%`,
          background: fillGradient || variantColors[variant].background,
          backgroundSize: striped ? '1rem 1rem' : 'auto'
        }"
      ></div>

      <!-- 条纹动画 -->
      <div
        v-if="striped"
        class="absolute top-0 left-0 w-full h-full rounded-full"
        :style="{
          backgroundImage: `linear-gradient(45deg, rgba(255, 255, 255, 0.15) 25%, transparent 25%, transparent 50%, rgba(255, 255, 255, 0.15) 50%, rgba(255, 255, 255, 0.15) 75%, transparent 75%, transparent)`,
          backgroundSize: '1rem 1rem',
          animation: 'progress-bar-stripes 1s linear infinite'
        }"
      ></div>
    </div>

    <!-- 描述文本 -->
    <div v-if="description || $slots.description" class="mt-2">
      <p class="text-xs text-gray-500 dark:text-gray-400">
        <slot name="description">{{ description }}</slot>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  value?: number
  max?: number
  min?: number
  variant?: 'primary' | 'secondary' | 'success' | 'warning' | 'danger' | 'info'
  size?: 'sm' | 'md' | 'lg' | 'xl'
  label?: string
  showLabel?: boolean
  showValue?: boolean
  description?: string
  indeterminate?: boolean
  striped?: boolean
  animated?: boolean
  className?: string
  trackColor?: string
  fillGradient?: string
  formatValue?: (value: number, percentage: number) => string
}

const props = withDefaults(defineProps<Props>(), {
  value: 0,
  max: 100,
  min: 0,
  variant: 'primary',
  size: 'md',
  label: '',
  showLabel: true,
  showValue: true,
  description: '',
  indeterminate: false,
  striped: false,
  animated: false,
  className: '',
  trackColor: '',
  fillGradient: '',
  formatValue: undefined
})

const variantColors = {
  primary: {
    background: 'linear-gradient(90deg, #0ea5e9 0%, #3b82f6 100%)',
    track: 'rgba(14, 165, 233, 0.1)'
  },
  secondary: {
    background: 'linear-gradient(90deg, #64748b 0%, #94a3b8 100%)',
    track: 'rgba(100, 116, 139, 0.1)'
  },
  success: {
    background: 'linear-gradient(90deg, #10b981 0%, #34d399 100%)',
    track: 'rgba(16, 185, 129, 0.1)'
  },
  warning: {
    background: 'linear-gradient(90deg, #f59e0b 0%, #fbbf24 100%)',
    track: 'rgba(245, 158, 11, 0.1)'
  },
  danger: {
    background: 'linear-gradient(90deg, #ef4444 0%, #f87171 100%)',
    track: 'rgba(239, 68, 68, 0.1)'
  },
  info: {
    background: 'linear-gradient(90deg, #06b6d4 0%, #22d3ee 100%)',
    track: 'rgba(6, 182, 212, 0.1)'
  }
}

const percentage = computed(() => {
  if (props.indeterminate) return 100
  const clampedValue = Math.max(props.min, Math.min(props.value, props.max))
  return ((clampedValue - props.min) / (props.max - props.min)) * 100
})

const formattedValue = computed(() => {
  if (props.formatValue) {
    return props.formatValue(props.value, percentage.value)
  }

  if (props.indeterminate) {
    return '处理中...'
  }

  return `${Math.round(percentage.value)}%`
})

const trackColor = computed(() => {
  if (props.trackColor) return props.trackColor
  return variantColors[props.variant].track
})
</script>

<style scoped>
@keyframes progress-bar-stripes {
  0% {
    background-position: 1rem 0;
  }
  100% {
    background-position: 0 0;
  }
}

.progress-bar-track {
  background: rgba(0, 0, 0, 0.1);
}

.dark .progress-bar-track {
  background: rgba(255, 255, 255, 0.1);
}

.indeterminate {
  animation: indeterminate 1.5s infinite linear;
  background-size: 200% 100%;
  background-image: linear-gradient(
    90deg,
    transparent 0%,
    currentColor 50%,
    transparent 100%
  );
}

@keyframes indeterminate {
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}

.striped {
  background-image: linear-gradient(
    45deg,
    rgba(255, 255, 255, 0.15) 25%,
    transparent 25%,
    transparent 50%,
    rgba(255, 255, 255, 0.15) 50%,
    rgba(255, 255, 255, 0.15) 75%,
    transparent 75%,
    transparent
  );
}
</style>