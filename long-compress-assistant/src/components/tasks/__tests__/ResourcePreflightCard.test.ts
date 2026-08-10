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
    expect(card.text()).toContain(label)
    expect(card.text()).toContain('800.0 MiB')
    expect(card.text()).toContain('100.0 MiB')
  })
})
