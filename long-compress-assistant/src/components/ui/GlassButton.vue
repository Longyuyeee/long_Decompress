<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    :class="[
      'glass-button',
      'rounded-lg',
      'px-4',
      'py-2',
      'font-medium',
      'transition-all',
      'duration-200',
      'backdrop-blur-md',
      'border',
      'flex',
      'items-center',
      'justify-center',
      'space-x-2',
      variant === 'primary' ? 'glass-button-primary' : '',
      variant === 'secondary' ? 'glass-button-secondary' : '',
      variant === 'danger' ? 'glass-button-danger' : '',
      size === 'sm' ? 'px-3 py-1.5 text-sm' : '',
      size === 'lg' ? 'px-6 py-3 text-lg' : '',
      size === 'xl' ? 'px-8 py-4 text-xl' : '',
      fullWidth ? 'w-full' : '',
      loading ? 'opacity-70 cursor-wait' : '',
      disabled ? 'opacity-50 cursor-not-allowed' : 'hover:scale-105 active:scale-95',
      className
    ]"
    @click="handleClick"
  >
    <template v-if="loading">
      <i class="pi pi-spin pi-spinner"></i>
      <span v-if="loadingText">{{ loadingText }}</span>
    </template>
    <template v-else>
      <i v-if="icon" :class="['pi', `pi-${icon}`, text ? 'mr-2' : '']"></i>
      <span v-if="text || $slots.default">
        <slot>{{ text }}</slot>
      </span>
    </template>
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  type?: 'button' | 'submit' | 'reset'
  variant?: 'default' | 'primary' | 'secondary' | 'danger'
  size?: 'sm' | 'md' | 'lg' | 'xl'
  text?: string
  icon?: string
  disabled?: boolean
  loading?: boolean
  loadingText?: string
  fullWidth?: boolean
  className?: string
}

const emit = defineEmits<{
  click: [event: MouseEvent]
}>()

const props = withDefaults(defineProps<Props>(), {
  type: 'button',
  variant: 'default',
  size: 'md',
  text: '',
  icon: '',
  disabled: false,
  loading: false,
  loadingText: '',
  fullWidth: false,
  className: ''
})

const handleClick = (event: MouseEvent) => {
  if (!props.disabled && !props.loading) {
    emit('click', event)
  }
}
</script>

<style scoped>
.glass-button {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.05) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: inherit;
  box-shadow:
    0 4px 16px 0 rgba(31, 38, 135, 0.2),
    inset 0 1px 0 0 rgba(255, 255, 255, 0.1);
}

.glass-button:hover:not(:disabled):not(.loading) {
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.2) 0%,
    rgba(255, 255, 255, 0.1) 100%
  );
  box-shadow:
    0 6px 20px 0 rgba(31, 38, 135, 0.3),
    inset 0 1px 0 0 rgba(255, 255, 255, 0.15);
}

.glass-button-primary {
  background: linear-gradient(
    135deg,
    rgba(14, 165, 233, 0.2) 0%,
    rgba(14, 165, 233, 0.1) 100%
  );
  border-color: rgba(14, 165, 233, 0.3);
  color: rgb(14, 165, 233);
}

.glass-button-primary:hover:not(:disabled):not(.loading) {
  background: linear-gradient(
    135deg,
    rgba(14, 165, 233, 0.3) 0%,
    rgba(14, 165, 233, 0.2) 100%
  );
}

.glass-button-secondary {
  background: linear-gradient(
    135deg,
    rgba(100, 116, 139, 0.2) 0%,
    rgba(100, 116, 139, 0.1) 100%
  );
  border-color: rgba(100, 116, 139, 0.3);
  color: rgb(100, 116, 139);
}

.glass-button-secondary:hover:not(:disabled):not(.loading) {
  background: linear-gradient(
    135deg,
    rgba(100, 116, 139, 0.3) 0%,
    rgba(100, 116, 139, 0.2) 100%
  );
}

.glass-button-danger {
  background: linear-gradient(
    135deg,
    rgba(239, 68, 68, 0.2) 0%,
    rgba(239, 68, 68, 0.1) 100%
  );
  border-color: rgba(239, 68, 68, 0.3);
  color: rgb(239, 68, 68);
}

.glass-button-danger:hover:not(:disabled):not(.loading) {
  background: linear-gradient(
    135deg,
    rgba(239, 68, 68, 0.3) 0%,
    rgba(239, 68, 68, 0.2) 100%
  );
}

.dark .glass-button {
  background: linear-gradient(
    135deg,
    rgba(0, 0, 0, 0.1) 0%,
    rgba(0, 0, 0, 0.05) 100%
  );
  border-color: rgba(255, 255, 255, 0.1);
}

.dark .glass-button:hover:not(:disabled):not(.loading) {
  background: linear-gradient(
    135deg,
    rgba(0, 0, 0, 0.2) 0%,
    rgba(0, 0, 0, 0.1) 100%
  );
}
</style>