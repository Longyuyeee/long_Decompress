import { expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import LzopRecoveryDialog from '../LzopRecoveryDialog.vue'
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn() }))

it('keeps matching basenames identifiable beneath long GNU directory names', async () => {
  const parent = '资料目录'.repeat(20)
  const paths = [`${parent}/readme.txt`, 'other/readme.txt']
  const view = mount(LzopRecoveryDialog, { props: { path: 'C:/source.tar.lzo', initial: {
    format: 'TAR.LZO', revision: 'a'.repeat(64), complete: false, stopReason: 'broken tail', stoppedOffset: 3000, availableVolumes: 1, ignoredVolumes: 0,
    entries: paths.map((name, id) => ({ id, name, bytes: 4, sha256: 'b'.repeat(64), isDirectory: false })),
  } }, global: { plugins: [createPinia()], stubs: { Teleport: true, Transition: false } } })
  await flushPromises()
  const rows = view.findAll('[data-recovery-member]')
  expect(rows.map(row => row.get('.member-copy strong').text())).toEqual(['readme.txt', 'readme.txt'])
  expect(rows.map(row => row.get('.member-copy small').text())).toEqual([parent, 'other'])
  expect(rows.map(row => row.get('.member-copy').attributes('title'))).toEqual(paths)
  expect(rows.every(row => (row.get('input').element as HTMLInputElement).checked)).toBe(true)
  view.unmount()
})
