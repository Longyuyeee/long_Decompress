import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ArchiveRecoveryDetails from '../ArchiveRecoveryDetails.vue'

describe('evidence-based recovery results', () => {
  it('separates verified files, missing volumes, failed checks and unknown content without claiming total completeness', async () => {
    const wrapper = mount(ArchiveRecoveryDetails, { props: { skippedEntries: ['legacy reason'], recoveredEntries: ['good.txt'], entryResults: [
      { name: 'good.txt', status: 'recovered', reason: '完整校验' },
      { name: '跨卷.bin', status: 'missing', reason: '涉及缺失卷' },
      { name: '坏文件.txt', status: 'checksum_failed', reason: 'CRC 校验失败' },
      { name: null, status: 'unknown', reason: '目录损坏，不能确认后续文件' },
    ] } })
    expect(wrapper.findAll('details')).toHaveLength(4)
    expect(wrapper.get('[data-testid="recovery-outcome-missing"]').text()).toContain('跨卷.bin')
    expect(wrapper.get('[data-testid="recovery-outcome-checksum_failed"]').text()).toContain('CRC 校验失败')
    expect(wrapper.get('[data-testid="recovery-outcome-unknown"]').text()).toContain('文件清单未知')
    expect(wrapper.text()).not.toContain('legacy reason')
    expect(wrapper.findAll('details[open]')).toHaveLength(0)
  })
  it('keeps older saved ZIP and RAR results readable', () => {
    const wrapper = mount(ArchiveRecoveryDetails, { props: { recoveredEntries: ['old.txt'], skippedEntries: ['old issue'] } })
    expect(wrapper.get('[data-testid="recovery-file-details"]').text()).toContain('old.txt')
    expect(wrapper.get('[data-testid="recovery-skipped-details"]').text()).toContain('old issue')
  })
})
