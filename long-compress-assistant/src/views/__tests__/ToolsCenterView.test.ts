import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent, h, onUnmounted, ref } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import productionRouter from '@/router'
import MainLayout from '@/components/layouts/MainLayout.vue'
import ToolsCenterView from '../ToolsCenterView.vue'

vi.mock('@tauri-apps/api/app', () => ({ getVersion: async () => '1.6.7' }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/window', () => ({ appWindow: { onFocusChanged: async () => () => {} } }))
vi.mock('@/stores/app', () => ({ useAppStore: () => ({ t: (key: string) => key }) }))
vi.mock('@/components/layouts/WindowTitleBar.vue', () => ({ default: { template: '<div />' } }))
vi.mock('@/components/ui/GlobalProgressBar.vue', () => ({ default: { template: '<div />' } }))
vi.mock('@/components/tasks/PendingDraftRecovery.vue', () => ({ default: { template: '<div />' } }))
vi.mock('@/components/archive/ZipEditRecoveryNotice.vue', () => ({ default: { template: '<div />' } }))

const wrappers: ReturnType<typeof mount>[] = []
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()) })

async function workspace(path: string) {
  const cleanup = vi.fn()
  const Tool = defineComponent({ setup() {
    const input = ref('')
    onUnmounted(cleanup)
    return () => h('input', { value: input.value, onInput: (event: Event) => { input.value = (event.target as HTMLInputElement).value } })
  } })
  const parent = productionRouter.options.routes.find(route => route.name === 'ToolsCenter')!
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { ...parent, component: ToolsCenterView, children: parent.children!.map(child => ({ ...child, component: Tool })) },
    { path: '/decompress', name: 'Decompress', component: { template: '<h1>解压中心</h1>' } },
  ] })
  await router.push(path); await router.isReady()
  const wrapper = mount(MainLayout, { global: { plugins: [router], stubs: { transition: false } } })
  wrappers.push(wrapper); await flushPromises()
  return { wrapper, router, cleanup }
}

describe('tool center navigation and retained work', () => {
  it('accepts all original URLs with query data and selects the sidebar parent and tool', async () => {
    const { wrapper, router } = await workspace('/compare?from=history')
    for (const [path, name] of [['compare','SourceComparison'],['workflows','Workflows'],['library','ArchiveLibrary'],['integrity','FileIntegrity']]) {
      await router.push(`/${path}?from=history`); await flushPromises()
      expect(router.currentRoute.value.query.from).toBe('history')
      expect(wrapper.get('[data-testid="nav-ToolsCenter"]').attributes('aria-current')).toBe('page')
      expect(wrapper.get(`[data-testid="tool-${name}"]`).attributes('aria-current')).toBe('page')
      expect(wrapper.findAll('aside nav>button')).toHaveLength(8)
    }
  })

  it('keeps the actual child instance and input while switching tools, then cleans up on leaving the center', async () => {
    const { wrapper, router, cleanup } = await workspace('/tools/compare')
    await wrapper.get('.tools-stage input').setValue('正在编辑的来源')
    await wrapper.get('[data-testid="tool-Workflows"]').trigger('click'); await flushPromises()
    expect(cleanup).not.toHaveBeenCalled()
    await wrapper.get('[data-testid="tool-SourceComparison"]').trigger('click'); await flushPromises()
    expect(wrapper.get<HTMLInputElement>('.tools-stage input').element.value).toBe('正在编辑的来源')
    await wrapper.get('[data-testid="nav-ToolsCenter"]').trigger('click'); await flushPromises()
    expect(router.currentRoute.value.name).toBe('SourceComparison')
    await router.push('/decompress'); await flushPromises()
    expect(cleanup).toHaveBeenCalledTimes(2)
  })

  it('opens the default tool from the center entry and preserves history back navigation', async () => {
    const { wrapper, router } = await workspace('/tools')
    expect(router.currentRoute.value.name).toBe('SourceComparison')
    await wrapper.get('[data-testid="tool-ArchiveLibrary"]').trigger('click'); await flushPromises()
    expect(router.currentRoute.value.path).toBe('/tools/library')
    router.back(); await flushPromises()
    expect(wrapper.get('[data-testid="tool-SourceComparison"]').attributes('aria-current')).toBe('page')
  })
})
