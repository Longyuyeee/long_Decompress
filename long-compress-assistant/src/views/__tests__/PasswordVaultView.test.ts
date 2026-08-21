import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import PasswordVaultView from '../PasswordVaultView.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({ exportPasswords: vi.fn(), importPasswords: vi.fn() }),
}))

const mountView = () => mount(PasswordVaultView, {
  global: {
    plugins: [createPinia()],
    stubs: { PasswordEntryModal: true, Transition: true },
  },
})

describe('PasswordVaultView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, string>) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'is_encrypted_password_service_unlocked') return false
      if (command === 'get_or_create_master_key') throw new Error('not initialized')
      if (command === 'unlock_encrypted_password_service') return args?.masterPassword === 'correct-password'
      if (command === 'list_encrypted_passwords' || command === 'list_password_groups') return []
      return undefined
    })
  })

  it('shows a stable skeleton until the first vault check finishes', async () => {
    let resolveStatus: ((value: boolean) => void) | undefined
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'load_app_settings') return Promise.resolve('{}')
      if (command === 'is_encrypted_password_service_unlocked') {
        return new Promise<boolean>(resolve => { resolveStatus = resolve })
      }
      if (command === 'list_encrypted_passwords' || command === 'list_password_groups') return Promise.resolve([])
      if (command === 'get_or_create_master_key') {
        return Promise.reject(new Error('not initialized'))
      }
      return Promise.resolve(undefined)
    })
    const wrapper = mountView()

    expect(wrapper.find('[aria-busy="true"]').exists()).toBe(true)
    expect(wrapper.text()).not.toContain('保险箱为空')
    resolveStatus?.(true)
    await flushPromises()

    expect(wrapper.find('[aria-busy="true"]').exists()).toBe(false)
  })

  it('does not expose manual lock or master-password controls when initialization fails', async () => {
    const wrapper = mountView()
    await flushPromises()

    expect(wrapper.find('input[type="text"]').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('密码保险箱暂时不可用')
    expect(wrapper.text()).toContain('重新加载')
    expect(wrapper.text()).not.toContain('锁定保险箱')
    expect(wrapper.text()).not.toContain('解锁保险箱')
    expect(wrapper.find('input[autocomplete="current-password"]').exists()).toBe(false)
  })

  it('retries automatic initialization with the installation key', async () => {
    const wrapper = mountView()
    await flushPromises()

    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'is_encrypted_password_service_unlocked') return false
      if (command === 'get_or_create_master_key') return 'installation-key'
      if (command === 'unlock_encrypted_password_service') return true
      if (command === 'list_encrypted_passwords' || command === 'list_password_groups') return []
      return undefined
    })
    await wrapper.findAll('button').find(button => button.text().includes('重新加载'))!.trigger('click')
    await flushPromises()

    expect(mocks.invoke).toHaveBeenCalledWith('unlock_encrypted_password_service', { masterPassword: 'installation-key' })
    expect(wrapper.text()).not.toContain('密码保险箱暂时不可用')
  })

  it('shows and hides an individual password beside its copy action', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'is_encrypted_password_service_unlocked') return true
      if (command === 'list_encrypted_passwords') {
        return [{
          id: 'entry-1',
          name: '测试密码',
          password: 'Secret!123',
          notes: '测试备注',
          tags: [],
          category: 'Other',
          strength: 'Medium',
          created_at: '2026-01-01T00:00:00.000Z',
          updated_at: '2026-01-01T00:00:00.000Z',
          favorite: false,
          use_count: 0,
          custom_fields: [],
        }]
      }
      if (command === 'list_password_groups') return []
      return undefined
    })
    const wrapper = mountView()
    await flushPromises()

    const columns = wrapper.findAll('colgroup').at(0)!.findAll('col')
    expect(columns.map(column => column.classes()[0])).toEqual([
      'vault-col-name',
      'vault-col-password',
      'vault-col-notes',
      'vault-col-usage',
      'vault-col-actions',
    ])
    expect(wrapper.get('[data-testid="vault-usage-header"]').classes()).toContain('whitespace-nowrap')
    expect(wrapper.text()).not.toContain('Secret!123')
    expect(wrapper.find('[aria-label="复制密码"]').exists()).toBe(true)
    await wrapper.find('[aria-label="显示密码"]').trigger('click')
    expect(wrapper.text()).toContain('Secret!123')
    expect(wrapper.find('[aria-label="隐藏密码"]').exists()).toBe(true)
  })

  it('refreshes backend usage and assigns it to the current local day before opening analytics', async () => {
    const now = new Date()
    const localDay = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
    let listCalls = 0
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'is_encrypted_password_service_unlocked') return true
      if (command === 'list_encrypted_passwords') {
        const fresh = listCalls++ > 0
        return [{
          id: 'entry-live-usage',
          name: '实时使用密码',
          password: 'Secret!123',
          notes: '',
          tags: [],
          category: 'Other',
          strength: 'Strong',
          created_at: '2026-01-01T00:00:00.000Z',
          updated_at: new Date().toISOString(),
          last_used: fresh ? new Date().toISOString() : null,
          favorite: false,
          use_count: fresh ? 1 : 0,
          usage_history: fresh ? { [localDay]: 1 } : {},
          custom_fields: [],
        }]
      }
      if (command === 'list_password_groups') return []
      return undefined
    })
    const wrapper = mountView()
    await flushPromises()

    await wrapper.get('[data-testid="vault-analytics-trigger"]').trigger('click')
    await flushPromises()
    const modal = document.querySelector('[data-testid="vault-analytics-modal"]')
    const sevenDayButton = modal?.querySelector('[data-testid="vault-range-7d"]') as HTMLButtonElement
    sevenDayButton.click()
    await flushPromises()
    const usageCounts = Array.from(modal?.querySelectorAll('[data-testid="vault-usage-day-count"]') || [])

    expect(listCalls).toBeGreaterThanOrEqual(2)
    expect(usageCounts.at(-1)?.textContent).toBe('1')
    expect(modal?.querySelector('[data-testid="vault-range-usage-total"]')?.textContent).toContain('1')
    wrapper.unmount()
  })

  it('opens the vault panorama and switches to entry lifecycle analytics', async () => {
    const today = new Date().toISOString()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'is_encrypted_password_service_unlocked') return true
      if (command === 'list_encrypted_passwords') {
        return [
          {
            id: 'entry-analytics-1',
            name: '生产环境密码',
            password: 'Secure!Password123',
            notes: '核心服务',
            tags: ['production'],
            category: 'Work',
            strength: 'Strong',
            created_at: '2025-01-01T00:00:00.000Z',
            updated_at: '2026-07-01T00:00:00.000Z',
            last_used: '2026-07-28T00:00:00.000Z',
            favorite: true,
            use_count: 12,
            custom_fields: [],
            usage_history: { '2026-07-28': 3 },
          },
          {
            id: 'entry-analytics-2',
            name: '备用密码',
            password: 'short',
            notes: '',
            tags: [],
            category: 'Other',
            strength: 'Weak',
            created_at: '2024-01-01T00:00:00.000Z',
            updated_at: '2024-01-01T00:00:00.000Z',
            last_used: null,
            favorite: false,
            use_count: 0,
            custom_fields: [],
          },
          {
            id: 'entry-analytics-3',
            name: '旧历史兼容密码',
            password: 'Legacy!Password123',
            notes: '',
            tags: [],
            category: 'Personal',
            strength: 'Strong',
            created_at: '2026-01-01T00:00:00.000Z',
            updated_at: today,
            last_used: today,
            favorite: false,
            use_count: 3,
            usage_history: {},
            custom_fields: [],
          },
        ]
      }
      if (command === 'list_password_groups') return []
      return undefined
    })
    const wrapper = mountView()
    await flushPromises()

    await wrapper.get('[data-testid="vault-analytics-trigger"]').trigger('click')
    await flushPromises()

    let modal = document.querySelector('[data-testid="vault-analytics-modal"]')
    expect(modal?.textContent).toContain('密码保险箱数据全景')
    expect(modal?.textContent).toContain('密码强度分布')
    expect(modal?.textContent).toContain('使用趋势')
    expect(modal?.textContent).toContain('风险雷达')
    expect(modal?.textContent).toContain('35 天活跃热力')
    expect(modal?.textContent).toContain('密码更新年龄')
    expect(modal?.textContent).toContain('可行动安全洞察')
    expect(modal?.querySelector('[data-testid="vault-activity-heatmap"]')).toBeTruthy()
    expect(modal?.querySelector('[data-testid="vault-age-breakdown"]')).toBeTruthy()
    expect(modal?.querySelector('[data-testid="vault-action-insights"]')?.textContent).toContain('弱密码需要升级')
    expect(modal?.textContent).toContain('长期使用画像')
    expect(modal?.textContent).toContain('保险箱使用时长')
    expect(modal?.textContent).toContain('历史月均使用')
    expect(modal?.querySelector('[data-testid="vault-range-7d"]')).toBeTruthy()
    expect(modal?.querySelector('[data-testid="vault-range-30d"]')).toBeTruthy()
    expect(modal?.querySelector('[data-testid="vault-range-90d"]')).toBeTruthy()
    const sevenDayButton = modal?.querySelector('[data-testid="vault-range-7d"]') as HTMLButtonElement
    sevenDayButton.click()
    await flushPromises()
    modal = document.querySelector('[data-testid="vault-analytics-modal"]')
    const usageCounts = Array.from(modal?.querySelectorAll('[data-testid="vault-usage-day-count"]') || [])
    expect(usageCounts[usageCounts.length - 1]?.textContent).toBe('1')
    const allRangeButton = modal?.querySelector('[data-testid="vault-range-all"]') as HTMLButtonElement
    allRangeButton.click()
    await flushPromises()
    modal = document.querySelector('[data-testid="vault-analytics-modal"]')
    expect(modal?.querySelector('[data-testid="vault-range-usage-total"]')?.textContent).toContain('4')
    expect(modal?.querySelector('[data-testid="vault-attention-count"]')?.textContent).toBe('1')

    ;(modal?.querySelector('[aria-label="关闭"]') as HTMLButtonElement)?.click()
    await flushPromises()
    await wrapper.get('[data-testid="vault-entry-usage"]').trigger('click')
    await flushPromises()

    modal = document.querySelector('[data-testid="vault-analytics-modal"]')
    expect(modal?.textContent).toContain('生产环境密码')
    expect(modal?.querySelector('[data-testid="vault-entry-lifecycle"]')).toBeTruthy()
    expect(modal?.textContent).toContain('保管天数')
    expect(modal?.textContent).toContain('正文长度')
  }, 12_000)
})

void consoleError
