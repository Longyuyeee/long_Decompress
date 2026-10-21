<script setup lang="ts">
import { computed } from 'vue'
import { taskScopes, taskScopeCounts, type TaskScope } from '@/utils/taskScope'
const props = defineProps<{ modelValue: TaskScope; statuses: (string | undefined)[] }>()
const emit = defineEmits<{ 'update:modelValue': [value: TaskScope] }>()
const counts = computed(() => taskScopeCounts(props.statuses))
</script>
<template>
  <div class="task-scope-bar" role="group" aria-label="按任务状态查看" data-testid="task-scope-bar">
    <button v-for="scope in taskScopes" :key="scope.id" type="button" :data-testid="`task-scope-${scope.id}`"
      :aria-pressed="modelValue === scope.id" :class="{ selected: modelValue === scope.id }"
      @click="emit('update:modelValue', scope.id)">
      <i :class="['pi', scope.icon]" aria-hidden="true"></i>{{ scope.label }}<span>{{ counts[scope.id] }}</span>
    </button>
  </div>
</template>
<style scoped>
.task-scope-bar { display:flex; flex-wrap:wrap; align-items:center; gap:.35rem; min-width:0; padding:.55rem .75rem; border-bottom:1px solid var(--border-subtle); }
button { display:inline-flex; align-items:center; gap:.4rem; min-height:2rem; padding:.35rem .65rem; border:1px solid transparent; border-radius:.65rem; color:var(--text-muted); font-size:.75rem; transition:background .15s,color .15s; }
button:hover { background:var(--bg-input); }
button.selected { color:var(--dynamic-accent); background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent); border-color:color-mix(in srgb,var(--dynamic-accent) 30%,transparent); }
button span { font-variant-numeric:tabular-nums; border-radius:.4rem; padding:.05rem .35rem; background:var(--bg-input); }
button:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
</style>
