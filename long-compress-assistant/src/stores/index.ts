/**
 * Pinia Store 导出文件
 *
 * 此文件导出所有可用的store，方便在组件中导入使用
 */

export { useAppStore } from './app'
export { useFileStore } from './file'
export { useUIStore } from './ui'

// 类型导出
export type { FileItem, DecompressTask, AppSettings } from './app'
export type { FileHistory, FavoriteFile } from './file'
export type { Notification, ModalState, Toast } from './ui'

/**
 * 使用示例：
 *
 * 1. 在组件中导入单个store：
 * import { useAppStore } from '@/stores'
 *
 * 2. 在组件中导入多个store：
 * import { useAppStore, useFileStore } from '@/stores'
 *
 * 3. 在组件中使用：
 * const appStore = useAppStore()
 * const fileStore = useFileStore()
 * const uiStore = useUIStore()
 *
 * 4. 访问状态：
 * const theme = appStore.currentTheme
 * const selectedFiles = fileStore.selectedFileItems
 * const sidebarOpen = uiStore.sidebarOpen
 *
 * 5. 调用方法：
 * appStore.updateSettings({ theme: 'dark' })
 * fileStore.addFile({ name: 'test.zip', path: '/path/to/file', size: 1024, type: 'zip' })
 * uiStore.showSuccess('操作成功')
 */

/**
 * Store 功能说明：
 *
 * 1. app.ts - 应用核心状态管理
 *    - 主题设置、语言配置
 *    - 解压任务管理
 *    - 应用设置管理
 *    - 本地存储集成
 *
 * 2. file.ts - 文件管理状态
 *    - 文件列表管理
 *    - 文件选择状态
 *    - 历史记录管理
 *    - 收藏夹管理
 *    - 文件工具方法
 *
 * 3. ui.ts - 用户界面状态
 *    - 侧边栏状态
 *    - 通知系统
 *    - 模态框管理
 *    - Toast提示
 *    - 加载状态
 *    - 主题管理
 */

/**
 * 最佳实践：
 *
 * 1. 在setup函数中使用store：
 * <script setup lang="ts">
 * import { useAppStore } from '@/stores'
 *
 * const appStore = useAppStore()
 *
 * // 使用计算属性
 * const currentTheme = computed(() => appStore.currentTheme)
 *
 * // 调用方法
 * const toggleTheme = () => {
 *   appStore.updateSettings({ theme: appStore.currentTheme === 'dark' ? 'light' : 'dark' })
 * }
 * </script>
 *
 * 2. 在模板中使用：
 * <template>
 *   <div :class="{ 'dark': appStore.currentTheme === 'dark' }">
 *     <!-- 内容 -->
 *   </div>
 * </template>
 *
 * 3. 监听store变化：
 * watch(() => appStore.currentTheme, (newTheme) => {
 *   console.log('主题已切换:', newTheme)
 * })
 *
 * 4. 重置store状态：
 * // 在组件卸载时重置（如果需要）
 * onUnmounted(() => {
 *   appStore.$reset()
 * })
 */