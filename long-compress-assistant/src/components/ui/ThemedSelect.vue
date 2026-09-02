<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'

export interface ThemedSelectOption { value: string, label: string, detail?: string }
const props = withDefaults(defineProps<{ modelValue: string, options: ThemedSelectOption[], disabled?: boolean, testId?: string, ariaLabel?: string, placement?: 'top' | 'bottom' }>(), { placement: 'bottom' })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const root = ref<HTMLElement | null>(null)
const optionElements = ref<HTMLButtonElement[]>([])
const open = ref(false)
const activeIndex = ref(0)
const selected = computed(() => props.options.find(option => option.value === props.modelValue) || props.options[0])

const focusActive = () => void nextTick(() => optionElements.value[activeIndex.value]?.focus())
const show = () => {
  if (props.disabled) return
  activeIndex.value = Math.max(0, props.options.findIndex(option => option.value === props.modelValue))
  open.value = true
  focusActive()
}
const close = () => { open.value = false }
const choose = (value: string) => { emit('update:modelValue', value); close(); void nextTick(() => root.value?.querySelector<HTMLButtonElement>('.select-trigger')?.focus()) }
const move = (delta: number) => {
  activeIndex.value = (activeIndex.value + delta + props.options.length) % props.options.length
  focusActive()
}
const onTriggerKeydown = (event: KeyboardEvent) => {
  if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) { event.preventDefault(); show() }
}
const onListKeydown = (event: KeyboardEvent) => {
  if (event.key === 'ArrowDown') { event.preventDefault(); move(1) }
  else if (event.key === 'ArrowUp') { event.preventDefault(); move(-1) }
  else if (event.key === 'Home') { event.preventDefault(); activeIndex.value = 0; focusActive() }
  else if (event.key === 'End') { event.preventDefault(); activeIndex.value = props.options.length - 1; focusActive() }
  else if (event.key === 'Escape' || event.key === 'Tab') close()
}
const onDocumentPointerDown = (event: PointerEvent) => { if (root.value && !root.value.contains(event.target as Node)) close() }
onMounted(() => document.addEventListener('pointerdown', onDocumentPointerDown))
onUnmounted(() => document.removeEventListener('pointerdown', onDocumentPointerDown))
</script>

<template>
  <div ref="root" class="themed-select" :class="[{ open, disabled }, `placement-${placement}`]">
    <button class="select-trigger" type="button" :disabled="disabled" :data-testid="testId" :aria-label="ariaLabel" aria-haspopup="listbox" :aria-expanded="open" @click="open ? close() : show()" @keydown="onTriggerKeydown">
      <span><strong>{{ selected?.label }}</strong><small v-if="selected?.detail">{{ selected.detail }}</small></span><i class="pi pi-chevron-down"></i>
    </button>
    <Transition name="select-pop">
      <div v-if="open" class="select-menu custom-scrollbar" role="listbox" :aria-label="ariaLabel" @keydown="onListKeydown">
        <button v-for="(option, index) in options" :key="option.value" :ref="element => { if (element) optionElements[index] = element as HTMLButtonElement }" type="button" role="option" :aria-selected="option.value === modelValue" :class="{ selected: option.value === modelValue }" @click="choose(option.value)">
          <i :class="option.value === modelValue ? 'pi pi-check' : ''"></i><span><strong>{{ option.label }}</strong><small v-if="option.detail">{{ option.detail }}</small></span>
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.themed-select{position:relative;min-width:0}.select-trigger{display:flex;width:100%;min-width:0;min-height:2.65rem;align-items:center;justify-content:space-between;gap:.65rem;border:1px solid var(--border-subtle);border-radius:.78rem;background:var(--bg-input);padding:.55rem .72rem;color:var(--text-content);text-align:left;outline:none;transition:border-color .18s ease,box-shadow .18s ease,background .18s ease}.select-trigger:hover,.themed-select.open .select-trigger{border-color:color-mix(in srgb,var(--dynamic-accent) 55%,var(--border-subtle));background:color-mix(in srgb,var(--dynamic-accent) 5%,var(--bg-input))}.select-trigger:focus-visible{box-shadow:0 0 0 3px color-mix(in srgb,var(--dynamic-accent) 16%,transparent)}.select-trigger span,.select-trigger strong,.select-trigger small{display:block;min-width:0}.select-trigger strong{overflow:hidden;font-size:.68rem;font-weight:850;text-overflow:ellipsis;white-space:nowrap}.select-trigger small{margin-top:.1rem;color:var(--text-muted);font-size:.52rem}.select-trigger>i{flex:0 0 auto;color:var(--text-muted);font-size:.65rem;transition:transform .18s ease}.themed-select.open .select-trigger>i{transform:rotate(180deg)}.select-menu{position:absolute;z-index:80;top:calc(100% + .35rem);right:0;left:0;max-height:13rem;overflow-y:auto;border:1px solid color-mix(in srgb,var(--dynamic-accent) 30%,var(--border-subtle));border-radius:.85rem;background:color-mix(in srgb,var(--bg-modal) 97%,transparent);padding:.35rem;box-shadow:0 22px 48px -24px rgb(0 0 0 / .72);backdrop-filter:blur(18px)}.placement-top .select-menu{top:auto;bottom:calc(100% + .35rem)}.select-menu button{display:grid;width:100%;grid-template-columns:1rem minmax(0,1fr);align-items:center;gap:.45rem;border-radius:.62rem;padding:.52rem .55rem;color:var(--text-content);text-align:left;outline:none}.select-menu button:hover,.select-menu button:focus-visible{background:color-mix(in srgb,var(--dynamic-accent) 10%,var(--bg-input))}.select-menu button.selected{color:var(--dynamic-accent)}.select-menu button>i{font-size:.62rem}.select-menu span,.select-menu strong,.select-menu small{display:block;min-width:0}.select-menu strong{font-size:.65rem;font-weight:850}.select-menu small{margin-top:.08rem;color:var(--text-muted);font-size:.5rem}.disabled{opacity:.55}.select-pop-enter-active,.select-pop-leave-active{transition:opacity .14s ease,transform .14s ease}.select-pop-enter-from,.select-pop-leave-to{opacity:0;transform:translateY(-.3rem) scale(.98)}
</style>
