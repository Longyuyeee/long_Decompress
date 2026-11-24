import { mount } from '@vue/test-utils'
import { expect, it, vi } from 'vitest'
import { nextTick, reactive } from 'vue'
import FilePaneList, { type FilePaneEntry } from '../FilePaneList.vue'

it('keeps the date text and tooltip identical for local and archived timestamps', async () => {
  const cases: Array<{ modified: FilePaneEntry['modified']; date?: Date }> = [
    { modified: 1704067200, date: new Date(1704067200000) },
    { modified: 1704067200000, date: new Date(1704067200000) },
    {
      modified: { secs_since_epoch: 1704067200, nanos_since_epoch: 950000000 },
      date: new Date(1704067200950),
    },
    { modified: '2024-01-01T09:30:02+08:00', date: new Date('2024-01-01T09:30:02+08:00') },
    { modified: null },
    { modified: undefined },
    { modified: '' },
    { modified: 'invalid' },
  ]
  const entries = cases.map(({ modified }, index) => ({
    path: 'entry-' + index,
    name: 'entry-' + index,
    size: 1,
    isDir: false,
    modified,
  }))
  const wrapper = mount(FilePaneList, { props: { entries, selected: new Set() } })
  const dates = () => wrapper.findAll('[role="row"]').map(row => {
    const cell = row.element.children[2]
    return [cell.textContent, cell.getAttribute('title')]
  })
  expect(dates()).toEqual(cases.map(({ date }) => {
    const label = date?.toLocaleString('zh-CN', { hour12: false }) ?? '—'
    return [label, label]
  }))

  await wrapper.setProps({ entries: [{ ...entries[0], modified: null }] })
  expect(dates()).toEqual([['—', '—']])
  wrapper.unmount()
})

it('updates selection and preview actions when a row is refreshed under the same path', async () => {
  const entry = {
    path: 'guide.txt',
    name: 'guide.txt',
    size: 12,
    isDir: false,
    preview: true,
    previewDisabled: true,
  }
  const wrapper = mount(FilePaneList, { props: { entries: [entry], selected: new Set() } })
  const row = () => wrapper.get('[role="row"]')
  const preview = () => row().get('[data-testid="file-manager-preview"]')

  expect(preview().attributes('disabled')).toBeDefined()
  await wrapper.setProps({ selected: new Set([entry.path]), selectionMode: true })
  expect(row().classes()).toContain('selected')
  expect((row().get('input').element as HTMLInputElement).checked).toBe(true)

  const refreshed = { ...entry, name: 'Guide.txt', size: 34, previewDisabled: false }
  await wrapper.setProps({ entries: [refreshed], selected: new Set(), selectionMode: false })
  expect(row().classes()).not.toContain('selected')
  expect(row().find('input').exists()).toBe(false)
  expect(row().get('strong').text()).toBe(refreshed.name)
  expect(row().element.children[1].textContent).toBe('34 B')
  expect(preview().attributes('disabled')).toBeUndefined()
  await preview().trigger('click')
  expect(wrapper.emitted('preview')).toEqual([[refreshed]])
  wrapper.unmount()
})

it('refreshes an existing row date when the system time zone or timestamp changes', async () => {
  const DateTimeFormat = Intl.DateTimeFormat
  let timeZone = 'UTC'
  const formatting = vi.spyOn(Intl, 'DateTimeFormat').mockImplementation(function (locales, options) {
    return new DateTimeFormat(locales, { ...options, timeZone })
  })
  const entry = reactive({
    path: 'guide.txt',
    name: 'guide.txt',
    size: 12,
    isDir: false,
    modified: { secs_since_epoch: 1704067200 },
  })
  const wrapper = mount(FilePaneList, { props: { entries: [entry], selected: new Set() } })
  const label = () => wrapper.get('[role="row"]').element.children[2].textContent
  const expected = () => new Date(entry.modified.secs_since_epoch * 1000)
    .toLocaleString('zh-CN', { hour12: false, timeZone })
  try {
    expect(label()).toBe(expected())
    const utcLabel = label()
    timeZone = 'Asia/Tokyo'
    await wrapper.setProps({ selected: new Set([entry.path]) })
    expect(label()).toBe(expected())
    expect(label()).not.toBe(utcLabel)

    entry.modified.secs_since_epoch += 3600
    await nextTick()
    expect(label()).toBe(expected())
  } finally {
    wrapper.unmount()
    formatting.mockRestore()
  }
})
