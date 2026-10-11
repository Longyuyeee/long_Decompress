export function archiveOutputLocationError(error: unknown): string {
  const message = String(error)
  if (message.startsWith('EXPLORER_PATH_UNAVAILABLE:not-found:')) {
    return '找不到记录中的文件，可能已移动或删除。可复制原路径查找；历史记录不会因此删除。'
  }
  if (message.startsWith('EXPLORER_PATH_UNAVAILABLE:permission-denied:')) {
    return '没有权限访问该文件。请检查文件或共享目录权限后重试。'
  }
  return '暂时无法定位产物，请检查磁盘、网络连接或文件管理器后重试。可复制原路径手动查找。'
}
