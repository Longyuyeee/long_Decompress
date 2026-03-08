import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import TaskList from '../TaskList.vue'
import type { DecompressTask } from '@/stores'

// Mock PrimeVue图标
vi.mock('primevue/icons', () => ({
  Filter: { template: '<i class="pi pi-filter"></i>' },
  ChevronDown: { template: '<i class="pi pi-chevron-down"></i>' },
  SortAmountDown: { template: '<i class="pi pi-sort-amount-down"></i>' },
  SortAmountUp: { template: '<i class="pi pi-sort-amount-up"></i>' },
  Refresh: { template: '<i class="pi pi-refresh"></i>' },
  Trash: { template: '<i class="pi pi-trash"></i>' },
  Inbox: { template: '<i class="pi pi-inbox"></i>' },
  Clock: { template: '<i class="pi pi-clock"></i>' },
  Spinner: { template: '<i class="pi pi-spinner"></i>' },
  Check: { template: '<i class="pi pi-check"></i>' },
  Times: { template: '<i class="pi pi-times"></i>' },
  FolderOpen: { template: '<i class="pi pi-folder-open"></i>' },
  Lock: { template: '<i class="pi pi-lock"></i>' },
  Cog: { template: '<i class="pi pi-cog"></i>' },
  Replay: { template: '<i class="pi pi-replay"></i>' },
  Pause: { template: '<i class="pi pi-pause"></i>' },
  ExclamationCircle: { template: '<i class="pi pi-exclamation-circle"></i>' },
}))

// Mock store
const mockAppStore = {
  decompressTasks: [] as DecompressTask[],
  activeTasks: [] as DecompressTask[],
  completedTasks: [] as DecompressTask[],
  totalProgress: 0,
  createDecompressTask: vi.fn(),
  updateTaskProgress: vi.fn(),
  markTaskAsError: vi.fn(),
  clearCompletedTasks: vi.fn(),
}

const mockFileStore = {
  files: [],
  selectedFiles: [],
  fileHistory: [],
  favoriteFiles: [],
  currentDirectory: '',
  isLoading: false,
  error: null,
  selectedFileItems: [],
  totalSelectedSize: 0,
  recentHistory: [],
  favoritesByTag: {},
  addFile: vi.fn(),
  removeFile: vi.fn(),
  clearFiles: vi.fn(),
  updateFileStatus: vi.fn(),
  selectFile: vi.fn(),
  deselectFile: vi.fn(),
  toggleFileSelection: vi.fn(),
  selectAllFiles: vi.fn(),
  deselectAllFiles: vi.fn(),
  addToHistory: vi.fn(),
  clearHistory: vi.fn(),
  addToFavorites: vi.fn(),
  removeFromFavorites: vi.fn(),
  updateFavoriteTags: vi.fn(),
  setCurrentDirectory: vi.fn(),
  formatFileSize: vi.fn().mockReturnValue('1.0 MB'),
  getFileExtension: vi.fn().mockReturnValue('zip'),
}

const mockUIStore = {
  sidebarOpen: true,
  notifications: [],
  modals: [],
  toasts: [],
  loading: false,
  loadingText: '',
  darkMode: false,
  currentView: 'home',
  unreadNotifications: 0,
  activeModals: [],
  hasActiveModals: false,
  isLoading: false,
  toggleSidebar: vi.fn(),
  openSidebar: vi.fn(),
  closeSidebar: vi.fn(),
  addNotification: vi.fn(),
  removeNotification: vi.fn(),
  markNotificationAsRead: vi.fn(),
  markAllNotificationsAsRead: vi.fn(),
  clearNotifications: vi.fn(),
  openModal: vi.fn(),
  closeModal: vi.fn(),
  removeModal: vi.fn(),
  closeAllModals: vi.fn(),
  showToast: vi.fn(),
  removeToast: vi.fn(),
  clearToasts: vi.fn(),
  startLoading: vi.fn(),
  stopLoading: vi.fn(),
  toggleDarkMode: vi.fn(),
  setDarkMode: vi.fn(),
  setCurrentView: vi.fn(),
  showSuccess: vi.fn(),
  showError: vi.fn(),
  showWarning: vi.fn(),
  showInfo: vi.fn(),
}

// Mock store模块
vi.mock('@/stores', () => ({
  useAppStore: () => mockAppStore,
  useFileStore: () => mockFileStore,
  useUIStore: () => mockUIStore,
}))

describe('TaskList.vue', () => {
  let pinia: ReturnType<typeof createPinia>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)

    // 重置mock状态
    vi.clearAllMocks()
    mockAppStore.decompressTasks = []
    mockAppStore.activeTasks = []
    mockAppStore.completedTasks = []
    mockAppStore.totalProgress = 0
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // 测试数据
  const createMockTask = (overrides = {}): DecompressTask => ({
    id: 'task-1',
    fileId: 'file-1',
    outputPath: '/output/path',
    password: undefined,
    options: {
      keepStructure: true,
      overwrite: false,
      deleteAfter: false,
    },
    status: 'pending',
    progress: 0,
    startTime: new Date('2024-03-08T10:00:00'),
    endTime: undefined,
    error: undefined,
    ...overrides,
  })

  describe('渲染测试', () => {
    it('应该正确渲染空状态', async () => {
      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 检查空状态显示
      expect(wrapper.text()).toContain('暂无任务')
      expect(wrapper.text()).toContain('还没有任何解压任务')

      // 检查统计信息
      expect(wrapper.text()).toContain('总任务数')
      expect(wrapper.text()).toContain('0')
    })

    it('应该正确渲染任务列表', async () => {
      // 设置mock数据
      const task = createMockTask()
      mockAppStore.decompressTasks = [task]
      mockAppStore.activeTasks = [task]

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 检查任务显示
      expect(wrapper.text()).toContain('文件_file-1')
      expect(wrapper.text()).toContain('等待中')
      expect(wrapper.text()).toContain('输出: /output/path')

      // 检查统计信息
      expect(wrapper.text()).toContain('总任务数')
      expect(wrapper.text()).toContain('1')
      expect(wrapper.text()).toContain('进行中')
      expect(wrapper.text()).toContain('1')
    })

    it('应该正确显示不同状态的任务', async () => {
      const tasks = [
        createMockTask({ id: 'task-1', status: 'pending' }),
        createMockTask({ id: 'task-2', status: 'processing', progress: 50 }),
        createMockTask({ id: 'task-3', status: 'completed', progress: 100, endTime: new Date('2024-03-08T10:05:00') }),
        createMockTask({ id: 'task-4', status: 'error', error: '解压失败' }),
      ]

      mockAppStore.decompressTasks = tasks
      mockAppStore.activeTasks = [tasks[1]] // processing
      mockAppStore.completedTasks = [tasks[2]] // completed

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 检查不同状态的显示
      expect(wrapper.text()).toContain('等待中')
      expect(wrapper.text()).toContain('解压进度')
      expect(wrapper.text()).toContain('50%')
      expect(wrapper.text()).toContain('已完成')
      expect(wrapper.text()).toContain('解压失败')

      // 检查统计信息
      expect(wrapper.text()).toContain('总任务数')
      expect(wrapper.text()).toContain('4')
      expect(wrapper.text()).toContain('进行中')
      expect(wrapper.text()).toContain('1')
      expect(wrapper.text()).toContain('已完成')
      expect(wrapper.text()).toContain('1')
      expect(wrapper.text()).toContain('失败')
      expect(wrapper.text()).toContain('1')
    })
  })

  describe('筛选功能测试', () => {
    it('应该支持按状态筛选任务', async () => {
      const tasks = [
        createMockTask({ id: 'task-1', status: 'pending' }),
        createMockTask({ id: 'task-2', status: 'processing' }),
        createMockTask({ id: 'task-3', status: 'completed' }),
      ]

      mockAppStore.decompressTasks = tasks
      mockAppStore.activeTasks = [tasks[1]]
      mockAppStore.completedTasks = [tasks[2]]

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 初始应该显示所有任务
      expect(wrapper.text()).toContain('全部任务')

      // 点击筛选按钮
      const filterButton = wrapper.find('button[aria-label="筛选任务"]')
      await filterButton.trigger('click')

      // 应该显示筛选菜单
      expect(wrapper.text()).toContain('进行中')
      expect(wrapper.text()).toContain('已完成')
      expect(wrapper.text()).toContain('失败')
    })
  })

  describe('排序功能测试', () => {
    it('应该支持按时间排序', async () => {
      const tasks = [
        createMockTask({ id: 'task-1', startTime: new Date('2024-03-08T10:00:00') }),
        createMockTask({ id: 'task-2', startTime: new Date('2024-03-08T11:00:00') }),
      ]

      mockAppStore.decompressTasks = tasks

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 初始应该是降序（最新优先）
      const sortButton = wrapper.find('button[aria-label="切换排序顺序"]')
      expect(sortButton.text()).toContain('pi-sort-amount-down')

      // 点击排序按钮
      await sortButton.trigger('click')

      // 应该切换为升序
      expect(sortButton.text()).toContain('pi-sort-amount-up')
    })
  })

  describe('操作功能测试', () => {
    it('应该支持刷新任务列表', async () => {
      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 点击刷新按钮
      const refreshButton = wrapper.find('button[aria-label="刷新任务列表"]')
      await refreshButton.trigger('click')

      // 应该调用刷新方法
      expect(mockUIStore.showSuccess).toHaveBeenCalledWith('任务列表已刷新')
    })

    it('应该支持清理已完成任务', async () => {
      const tasks = [
        createMockTask({ id: 'task-1', status: 'completed' }),
        createMockTask({ id: 'task-2', status: 'processing' }),
      ]

      mockAppStore.decompressTasks = tasks
      mockAppStore.completedTasks = [tasks[0]]

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 点击清理按钮
      const clearButton = wrapper.find('button[aria-label="清理已完成的任务"]')
      await clearButton.trigger('click')

      // 应该调用清理方法
      expect(mockAppStore.clearCompletedTasks).toHaveBeenCalled()
      expect(mockUIStore.showSuccess).toHaveBeenCalledWith('已完成的任务已清理')
    })

    it('应该支持删除单个任务', async () => {
      const task = createMockTask()
      mockAppStore.decompressTasks = [task]

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 点击删除按钮
      const deleteButton = wrapper.find('button[aria-label="删除此任务"]')
      await deleteButton.trigger('click')

      // 应该显示成功提示
      expect(mockUIStore.showSuccess).toHaveBeenCalledWith('任务已删除')
    })

    it('应该支持取消进行中的任务', async () => {
      const task = createMockTask({ status: 'processing' })
      mockAppStore.decompressTasks = [task]
      mockAppStore.activeTasks = [task]

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 点击取消按钮
      const cancelButton = wrapper.find('button[aria-label="取消此任务"]')
      await cancelButton.trigger('click')

      // 应该调用取消方法
      expect(mockAppStore.markTaskAsError).toHaveBeenCalledWith('task-1', '任务已取消')
      expect(mockUIStore.showWarning).toHaveBeenCalledWith('任务已取消')
    })
  })

  describe('批量操作测试', () => {
    it('应该支持批量选择任务', async () => {
      const tasks = [
        createMockTask({ id: 'task-1' }),
        createMockTask({ id: 'task-2' }),
      ]

      mockAppStore.decompressTasks = tasks

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 初始不应该显示批量操作栏
      expect(wrapper.text()).not.toContain('已选择')

      // 注意：实际的批量选择功能需要在组件中添加实现
      // 这里只是测试框架
    })
  })

  describe('错误处理测试', () => {
    it('应该正确显示错误任务的信息', async () => {
      const task = createMockTask({
        status: 'error',
        error: '文件损坏，无法解压'
      })

      mockAppStore.decompressTasks = [task]

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 检查错误信息显示
      expect(wrapper.text()).toContain('解压失败')
      expect(wrapper.text()).toContain('文件损坏，无法解压')

      // 检查重试按钮
      const retryButton = wrapper.find('button[aria-label="重试此任务"]')
      expect(retryButton.exists()).toBe(true)
    })
  })

  describe('响应式设计测试', () => {
    it('应该在不同屏幕尺寸下正确渲染', async () => {
      const task = createMockTask()
      mockAppStore.decompressTasks = [task]

      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 检查响应式类是否存在
      expect(wrapper.html()).toContain('grid grid-cols-1 sm:grid-cols-2')
      expect(wrapper.html()).toContain('grid grid-cols-1 sm:grid-cols-4')

      // 检查隐藏类（移动端/桌面端）
      expect(wrapper.html()).toContain('hidden sm:inline')
    })
  })

  describe('可访问性测试', () => {
    it('应该包含必要的ARIA标签', async () => {
      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 检查ARIA标签
      expect(wrapper.find('button[aria-label="筛选任务"]').exists()).toBe(true)
      expect(wrapper.find('button[aria-label="切换排序顺序"]').exists()).toBe(true)
      expect(wrapper.find('button[aria-label="刷新任务列表"]').exists()).toBe(true)
      expect(wrapper.find('button[aria-label="清理已完成的任务"]').exists()).toBe(true)
    })

    it('应该支持键盘导航', async () => {
      const wrapper = mount(TaskList, {
        global: {
          plugins: [pinia],
        },
      })

      await flushPromises()

      // 检查焦点管理类
      expect(wrapper.html()).toContain('focus:ring-2')
      expect(wrapper.html()).toContain('focus:outline-none')
    })
  })
})