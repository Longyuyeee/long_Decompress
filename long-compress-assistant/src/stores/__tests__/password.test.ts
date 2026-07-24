import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  PasswordCategory,
  PasswordStrength,
  usePasswordStore,
  type PasswordEntry,
} from '../password'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

const entry = (overrides: Partial<PasswordEntry> = {}): PasswordEntry => ({
  id: 'entry-1',
  name: 'Work archive',
  username: 'alice',
  password: 'Secret!123',
  url: null,
  notes: null,
  tags: ['backup'],
  category: PasswordCategory.Work,
  strength: PasswordStrength.Strong,
  created_at: '2026-01-01T00:00:00.000Z',
  updated_at: '2026-01-01T00:00:00.000Z',
  last_used: null,
  expires_at: null,
  favorite: false,
  use_count: 0,
  custom_fields: [],
  ...overrides,
})

describe('Password Store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.invoke.mockReset()
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, any>) => {
      if (command === 'update_encrypted_password') return args?.entry
      return undefined
    })
  })

  it('loads vault entries and groups after a successful unlock check', async () => {
    const loaded = entry()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'is_encrypted_password_service_unlocked') return true
      if (command === 'list_encrypted_passwords') return [loaded]
      if (command === 'list_password_groups') return []
    })
    const store = usePasswordStore()

    await store.checkUnlockStatus()

    expect(store.isUnlocked).toBe(true)
    expect(store.entries).toEqual([loaded])
    expect(store.isLoading).toBe(false)
    expect(store.isInitialized).toBe(true)
  })

  it('filters, sorts, paginates, and clears local search state without mutating entries', () => {
    const store = usePasswordStore()
    store.entries = [
      entry({ id: 'b', name: 'Beta', favorite: true, use_count: 2 }),
      entry({ id: 'a', name: 'Alpha', favorite: false, use_count: 8 }),
    ]

    store.setSearchFilters({ favoritesOnly: true })
    expect(store.filteredEntries.map(item => item.id)).toEqual(['b'])

    store.clearSearchFilters()
    store.setSort('name', false)
    expect(store.filteredEntries.map(item => item.name)).toEqual(['Alpha', 'Beta'])
    expect(store.entries.map(item => item.name)).toEqual(['Beta', 'Alpha'])
    expect(store.totalPages).toBe(1)
  })

  it('persists favorite and usage changes through the encrypted vault command', async () => {
    const store = usePasswordStore()
    store.entries = [entry()]

    await store.toggleFavorite('entry-1')
    const password = await store.usePassword('entry-1')

    expect(password).toBe('Secret!123')
    expect(store.entries[0].favorite).toBe(true)
    expect(store.entries[0].use_count).toBe(1)
    expect(store.entries[0].last_used).toBeTruthy()
    expect(mocks.invoke).toHaveBeenCalledWith('update_encrypted_password', expect.objectContaining({ id: 'entry-1' }))
  })

  it('deletes all selected entries and clears the selection', async () => {
    const store = usePasswordStore()
    store.entries = [entry({ id: 'one' }), entry({ id: 'two' }), entry({ id: 'three' })]
    store.selectedPasswords = ['one', 'three']

    await store.deleteSelectedPasswords()

    expect(store.entries.map(item => item.id)).toEqual(['two'])
    expect(store.selectedPasswords).toEqual([])
    expect(mocks.invoke).toHaveBeenCalledWith('delete_encrypted_password', { id: 'one' })
    expect(mocks.invoke).toHaveBeenCalledWith('delete_encrypted_password', { id: 'three' })
  })

  it('prioritizes filename matches and removes duplicate candidate passwords', () => {
    const store = usePasswordStore()
    store.entries = [
      entry({ id: 'other', name: 'General', password: 'same', last_used: '2026-02-01T00:00:00.000Z' }),
      entry({ id: 'match', name: 'project backup', password: 'project-pass', use_count: 5 }),
      entry({ id: 'duplicate', name: 'Old', password: 'same' }),
    ]

    expect(store.findCandidatePasswords('project.zip')).toEqual(['project-pass', 'same'])
  })

  it('assesses empty, weak, and strong passwords', async () => {
    const store = usePasswordStore()

    expect((await store.assessPasswordStrength('')).score).toBe(0)
    expect((await store.assessPasswordStrength('password')).score).toBeLessThan(50)
    expect((await store.assessPasswordStrength('Long!Secure#Password123')).score).toBeGreaterThanOrEqual(90)
  })
})
