import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ResourcePreflightCard from '../ResourcePreflightCard.vue'
import type { ResourcePreflightReport } from '@/types/resourcePreflight'

const report = (status: ResourcePreflightReport['status']): ResourcePreflightReport => ({
  operation: 'decompression',
  outputPath: 'C:/output',
  probePath: 'C:/output',
  mountPoint: 'C:/',
  fileSystem: 'NTFS',
  location: 'local',
  medium: 'ssd',
  totalBytes: 1_000_000_000,
  availableBytes: 800 * 1024 * 1024,
  estimatedOutputBytes: 100 * 1024 * 1024,
  requiredBytes: 228 * 1024 * 1024,
  reserveBytes: 128 * 1024 * 1024,
  estimateSource: 'archive_metadata',
  estimateReliable: true,
  status,
  canStart: status !== 'blocked',
  summary: status === 'blocked' ? '目标盘空间不足' : '目标盘空间满足当前估算',
  warnings: status === 'warning' ? ['网络位置可能影响吞吐'] : [],
})

describe('ResourcePreflightCard', () => {
  it('stays hidden until a task has a preflight report', () => {
    expect(mount(ResourcePreflightCard).find('[data-testid="resource-preflight-card"]').exists()).toBe(false)
  })

  it.each([
    ['ready', '已通过'],
    ['warning', '需留意'],
    ['blocked', '已阻止'],
  ] as const)('renders the %s resource state without horizontal content', (status, label) => {
    const wrapper = mount(ResourcePreflightCard, { props: { report: report(status) } })
    const card = wrapper.get('[data-testid="resource-preflight-card"]')
    expect(card.classes()).toContain(`is-${status}`)
    expect(card.text()).toContain('目标存储预检')
    expect(card.text()).toContain(label)
    expect(card.text()).toContain('800.0 MiB')
    expect(card.text()).toContain('100.0 MiB')
    expect(wrapper.get('[data-testid="resource-preflight-metrics"]').classes()).toContain('grid-cols-2')
    expect(wrapper.get('[data-testid="resource-preflight-location"]').text()).toBe('目标位置本地磁盘')
    expect(wrapper.get('[data-testid="resource-preflight-medium"]').text()).toBe('存储介质SSD')
    expect(wrapper.get('[data-testid="resource-preflight-available"]').text()).toBe('剩余可用800.0 MiB')
    expect(wrapper.get('[data-testid="resource-preflight-estimated"]').text()).toBe('预计占用100.0 MiB')
  })

  it('reduces the decompression detail preflight to one non-wrapping status row', () => {
    const wrapper = mount(ResourcePreflightCard, { props: { report: report('ready'), compact: true } })
    const card = wrapper.get('[data-testid="resource-preflight-card"]')
    expect(card.classes()).toContain('is-compact')
    expect(card.text()).toContain('存储预检')
    expect(card.text()).toContain('已通过')
    expect(card.text()).toContain('可用 800.0 MiB')
    expect(card.text()).not.toContain('目标存储预检')
    expect(card.text()).not.toContain('预计占用')
  })
})
