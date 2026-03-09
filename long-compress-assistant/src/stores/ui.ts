import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface Notification {
  id: string
  type: 'info' | 'success' | 'warning' | 'error'
  title: string
  message: string
  timestamp: Date
  read: boolean
  duration?: number // 自动消失时间（毫秒），undefined表示不自动消�?
}

export interface ModalState {
  id: string
  component: string
  props?: Record<string, any>
  isOpen: boolean
}

export interface Toast {
  id: string
  type: 'info' | 'success' | 'warning' | 'error'
  message: string
  duration: number
}

export const useUIStore = defineStore('ui', () => {
  // 状�?
  const sidebarOpen = ref(true)
  const notifications = ref<Notification[]>([])
  const modals = ref<ModalState[]>([])
  const toasts = ref<Toast[]>([])
  const loading = ref(false)
  const loadingText = ref('')
  const darkMode = ref(false)
  const currentView = ref('home')

  // 计算属�?
  const unreadNotifications = computed(() => {
    return notifications.value.filter(notification => !notification.read).length
  })

  const activeModals = computed(() => {
    return modals.value.filter(modal => modal.isOpen)
  })

  const hasActiveModals = computed(() => {
    return activeModals.value.length > 0
  })

  const isLoading = computed(() => {
    return loading.value
  })

  // 方法 - 侧边�?
  const toggleSidebar = () => {
    sidebarOpen.value = !sidebarOpen.value
    localStorage.setItem('sidebar-open', sidebarOpen.value.toString())
  }

  const openSidebar = () => {
    sidebarOpen.value = true
    localStorage.setItem('sidebar-open', 'true')
  }

  const closeSidebar = () => {
    sidebarOpen.value = false
    localStorage.setItem('sidebar-open', 'false')
  }

  // 方法 - 通知
  const addNotification = (
    type: Notification['type'],
    title: string,
    message: string,
    duration?: number
  ) => {
    const notification: Notification = {
      id: generateId(),
      type,
      title,
      message,
      timestamp: new Date(),
      read: false,
      duration
    }

    notifications.value.unshift(notification)

    // 保持通知不超�?0�?
    if (notifications.value.length > 50) {
      notifications.value = notifications.value.slice(0, 50)
    }

    // 如果设置了持续时间，自动移除
    if (duration) {
      setTimeout(() => {
        removeNotification(notification.id)
      }, duration)
    }

    saveNotificationsToStorage()
  }

  const removeNotification = (id: string) => {
    notifications.value = notifications.value.filter(notification => notification.id !== id)
    saveNotificationsToStorage()
  }

  const markNotificationAsRead = (id: string) => {
    const notification = notifications.value.find(n => n.id === id)
    if (notification) {
      notification.read = true
      saveNotificationsToStorage()
    }
  }

  const markAllNotificationsAsRead = () => {
    notifications.value.forEach(notification => {
      notification.read = true
    })
    saveNotificationsToStorage()
  }

  const clearNotifications = () => {
    notifications.value = []
    localStorage.removeItem('notifications')
  }

  // 方法 - 模态框
  const openModal = (component: string, props?: Record<string, any>) => {
    const modalId = generateId()
    const modal: ModalState = {
      id: modalId,
      component,
      props,
      isOpen: true
    }

    modals.value.push(modal)
    return modalId
  }

  const closeModal = (id: string) => {
    const modal = modals.value.find(m => m.id === id)
    if (modal) {
      modal.isOpen = false
    }
  }

  const removeModal = (id: string) => {
    modals.value = modals.value.filter(modal => modal.id !== id)
  }

  const closeAllModals = () => {
    modals.value.forEach(modal => {
      modal.isOpen = false
    })
  }

  // 方法 - Toast提示
  const showToast = (type: Toast['type'], message: string, duration: number = 3000) => {
    const toast: Toast = {
      id: generateId(),
      type,
      message,
      duration
    }

    toasts.value.push(toast)

    // 自动移除
    setTimeout(() => {
      removeToast(toast.id)
    }, duration)

    // 保持Toast不超�?�?
    if (toasts.value.length > 5) {
      toasts.value = toasts.value.slice(1)
    }
  }

  const removeToast = (id: string) => {
    toasts.value = toasts.value.filter(toast => toast.id !== id)
  }

  const clearToasts = () => {
    toasts.value = []
  }

  // 方法 - 加载状�?
  const startLoading = (text: string = '加载�?..') => {
    loading.value = true
    loadingText.value = text
  }

  const stopLoading = () => {
    loading.value = false
    loadingText.value = ''
  }

  // 方法 - 主题
  const toggleDarkMode = () => {
    darkMode.value = !darkMode.value
    updateThemeClass()
    localStorage.setItem('dark-mode', darkMode.value.toString())
  }

  const setDarkMode = (enabled: boolean) => {
    darkMode.value = enabled
    updateThemeClass()
    localStorage.setItem('dark-mode', enabled.toString())
  }

  const updateThemeClass = () => {
    if (darkMode.value) {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
  }

  // 方法 - 视图管理
  const setCurrentView = (view: string) => {
    currentView.value = view
  }

  // 存储相关方法
  const saveNotificationsToStorage = () => {
    try {
      const notificationsData = notifications.value.map(notification => ({
        ...notification,
        timestamp: notification.timestamp.toISOString()
      }))
      localStorage.setItem('notifications', JSON.stringify(notificationsData))
    } catch (err) {
      console.error('Failed to save notifications to storage:', err)
    }
  }

  const loadNotificationsFromStorage = () => {
    try {
      const savedNotifications = localStorage.getItem('notifications')
      if (savedNotifications) {
        const notificationsData = JSON.parse(savedNotifications)
        notifications.value = notificationsData.map((item: any) => ({
          ...item,
          timestamp: new Date(item.timestamp)
        }))
      }
    } catch (err) {
      console.error('Failed to load notifications from storage:', err)
    }
  }

  // 辅助函数
  const generateId = () => {
    return Date.now().toString(36) + Math.random().toString(36).substr(2)
  }

  // 工具方法
  const showSuccess = (message: string, title: string = '成功') => {
    addNotification('success', title, message, 5000)
    showToast('success', message)
  }

  const showError = (message: string, title: string = '错误') => {
    addNotification('error', title, message, 8000)
    showToast('error', message)
  }

  const showWarning = (message: string, title: string = '警告') => {
    addNotification('warning', title, message, 6000)
    showToast('warning', message)
  }

  const showInfo = (message: string, title: string = '提示') => {
    addNotification('info', title, message, 4000)
    showToast('info', message)
  }

  // 初始�?
  const initialize = () => {
    // 加载通知
    loadNotificationsFromStorage()

    // 加载侧边栏状�?
    const savedSidebarState = localStorage.getItem('sidebar-open')
    if (savedSidebarState !== null) {
      sidebarOpen.value = savedSidebarState === 'true'
    }

    // 加载主题设置
    const savedDarkMode = localStorage.getItem('dark-mode')
    if (savedDarkMode !== null) {
      darkMode.value = savedDarkMode === 'true'
      updateThemeClass()
    } else {
      // 检查系统主题偏�?
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
      darkMode.value = prefersDark
      updateThemeClass()
    }

    // 监听系统主题变化
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
      const savedTheme = localStorage.getItem('dark-mode')
      if (savedTheme === null) {
        // 如果没有手动设置过主题，跟随系统
        darkMode.value = e.matches
        updateThemeClass()
      }
    })
  }

  // 执行初始�?
  initialize()

  return {
    // 状�?
    sidebarOpen,
    notifications,
    modals,
    toasts,
    loading,
    loadingText,
    darkMode,
    currentView,

    // 计算属�?
    unreadNotifications,
    activeModals,
    hasActiveModals,
    isLoading,

    // 侧边栏方�?
    toggleSidebar,
    openSidebar,
    closeSidebar,

    // 通知方法
    addNotification,
    removeNotification,
    markNotificationAsRead,
    markAllNotificationsAsRead,
    clearNotifications,

    // 模态框方法
    openModal,
    closeModal,
    removeModal,
    closeAllModals,

    // Toast方法
    showToast,
    removeToast,
    clearToasts,

    // 加载状态方�?
    startLoading,
    stopLoading,

    // 主题方法
    toggleDarkMode,
    setDarkMode,

    // 视图管理方法
    setCurrentView,

    // 工具方法
    showSuccess,
    showError,
    showWarning,
    showInfo,

    // 存储方法
    saveNotificationsToStorage,
    loadNotificationsFromStorage
  }
})
