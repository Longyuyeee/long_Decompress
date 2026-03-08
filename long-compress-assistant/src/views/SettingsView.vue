<template>
  <div class="max-w-4xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-8">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-white">设置</h1>
      <p class="text-gray-600 dark:text-gray-400">配置应用程序选项</p>
    </div>

      <main class="space-y-6">
        <!-- 常规设置 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">常规设置</h2>

          <div class="space-y-6">
            <!-- 主题设置 -->
            <div>
              <h3 class="font-medium text-gray-900 dark:text-white mb-4">主题</h3>
              <div class="grid grid-cols-3 gap-4">
                <button @click="settings.theme = 'light'"
                        class="p-4 rounded-lg border-2 text-center transition-all"
                        :class="settings.theme === 'light' ? 'border-primary bg-primary/10' : 'border-gray-200 dark:border-gray-700 hover:border-gray-300'">
                  <i class="pi pi-sun text-xl mb-2 block"></i>
                  <span class="font-medium">浅色</span>
                </button>
                <button @click="settings.theme = 'dark'"
                        class="p-4 rounded-lg border-2 text-center transition-all"
                        :class="settings.theme === 'dark' ? 'border-primary bg-primary/10' : 'border-gray-200 dark:border-gray-700 hover:border-gray-300'">
                  <i class="pi pi-moon text-xl mb-2 block"></i>
                  <span class="font-medium">深色</span>
                </button>
                <button @click="settings.theme = 'auto'"
                        class="p-4 rounded-lg border-2 text-center transition-all"
                        :class="settings.theme === 'auto' ? 'border-primary bg-primary/10' : 'border-gray-200 dark:border-gray-700 hover:border-gray-300'">
                  <i class="pi pi-desktop text-xl mb-2 block"></i>
                  <span class="font-medium">自动</span>
                </button>
              </div>
            </div>

            <!-- 语言设置 -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                语言
              </label>
              <select v-model="settings.language"
                      class="w-full glass-input">
                <option value="zh-CN">简体中文</option>
                <option value="en-US">English</option>
                <option value="ja-JP">日本語</option>
                <option value="ko-KR">한국어</option>
              </select>
            </div>

            <!-- 启动选项 -->
            <div class="space-y-3">
              <h3 class="font-medium text-gray-900 dark:text-white">启动选项</h3>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.startMinimized" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">启动时最小化到系统托盘</span>
              </label>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.autoCheckUpdates" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">自动检查更新</span>
              </label>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.showWelcome" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">每次启动显示欢迎页面</span>
              </label>
            </div>
          </div>
        </div>

        <!-- 解压设置 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">解压设置</h2>

          <div class="space-y-6">
            <!-- 默认输出目录 -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                默认输出目录
              </label>
              <div class="flex space-x-2">
                <input type="text" v-model="settings.defaultOutputPath"
                       class="flex-1 glass-input"
                       placeholder="选择默认输出目录">
                <button class="glass-button px-4">
                  <i class="pi pi-folder-open"></i>
                </button>
              </div>
            </div>

            <!-- 解压选项 -->
            <div class="space-y-3">
              <h3 class="font-medium text-gray-900 dark:text-white">默认解压选项</h3>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.defaultKeepStructure" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">保持目录结构</span>
              </label>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.defaultOverwrite" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">覆盖已存在文件</span>
              </label>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.defaultDeleteAfter" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">解压后删除原文件</span>
              </label>
            </div>

            <!-- 并发设置 -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                最大并发解压任务数
              </label>
              <div class="flex items-center space-x-4">
                <input type="range" v-model="settings.maxConcurrentTasks"
                       min="1" max="8" step="1"
                       class="flex-1">
                <span class="font-medium w-8 text-center">{{ settings.maxConcurrentTasks }}</span>
              </div>
              <p class="text-gray-500 dark:text-gray-400 text-sm mt-1">
                同时处理的最大文件数，数值越大速度越快但占用更多资源
              </p>
            </div>
          </div>
        </div>

        <!-- 安全设置 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">安全设置</h2>

          <div class="space-y-6">
            <!-- 文件扫描 -->
            <div class="space-y-3">
              <h3 class="font-medium text-gray-900 dark:text-white">文件安全检查</h3>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.scanForViruses" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">解压前扫描病毒</span>
              </label>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.checkFileExtensions" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">检查可疑文件扩展名</span>
              </label>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.warnLargeFiles" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">大文件解压前警告</span>
              </label>
            </div>

            <!-- 密码管理 -->
            <div>
              <h3 class="font-medium text-gray-900 dark:text-white mb-3">密码管理</h3>
              <label class="flex items-center mb-3">
                <input type="checkbox" v-model="settings.savePasswords" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">记住常用解压密码</span>
              </label>
              <div v-if="settings.savePasswords" class="ml-6 space-y-2">
                <label class="flex items-center">
                  <input type="checkbox" v-model="settings.encryptPasswords" class="mr-3">
                  <span class="text-gray-700 dark:text-gray-300">加密存储密码</span>
                </label>
                <label class="flex items-center">
                  <input type="checkbox" v-model="settings.autoClearPasswords" class="mr-3">
                  <span class="text-gray-700 dark:text-gray-300">退出时清除密码</span>
                </label>
              </div>
            </div>

            <!-- 隐私设置 -->
            <div class="space-y-3">
              <h3 class="font-medium text-gray-900 dark:text-white">隐私设置</h3>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.collectUsageData" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">收集匿名使用数据</span>
              </label>
              <label class="flex items-center">
                <input type="checkbox" v-model="settings.sendCrashReports" class="mr-3">
                <span class="text-gray-700 dark:text-gray-300">发送崩溃报告</span>
              </label>
            </div>
          </div>
        </div>

        <!-- 高级设置 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">高级设置</h2>

          <div class="space-y-6">
            <!-- 缓存设置 -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                缓存大小限制 (MB)
              </label>
              <div class="flex items-center space-x-4">
                <input type="range" v-model="settings.cacheSize"
                       min="50" max="1000" step="50"
                       class="flex-1">
                <span class="font-medium w-16 text-center">{{ settings.cacheSize }} MB</span>
              </div>
              <p class="text-gray-500 dark:text-gray-400 text-sm mt-1">
                用于临时文件和解压缓存的磁盘空间
              </p>
            </div>

            <!-- 日志级别 -->
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                日志级别
              </label>
              <select v-model="settings.logLevel"
                      class="w-full glass-input">
                <option value="error">仅错误</option>
                <option value="warn">警告和错误</option>
                <option value="info">信息（推荐）</option>
                <option value="debug">调试</option>
                <option value="trace">跟踪</option>
              </select>
            </div>

            <!-- 重置选项 -->
            <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
              <h3 class="font-medium text-gray-900 dark:text-white mb-3">重置选项</h3>
              <div class="flex space-x-4">
                <button @click="resetSettings"
                        class="glass-button px-4 py-2 text-red-600 hover:text-red-700">
                  <i class="pi pi-refresh mr-2"></i>
                  重置所有设置
                </button>
                <button @click="clearCache"
                        class="glass-button px-4 py-2">
                  <i class="pi pi-trash mr-2"></i>
                  清除缓存
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 保存按钮 -->
        <div class="flex justify-end space-x-4">
          <button @click="cancel"
                  class="glass-button px-6 py-3">
            取消
          </button>
          <button @click="saveSettings"
                  class="glass-button-primary px-6 py-3">
            <i class="pi pi-save mr-2"></i>
            保存设置
          </button>
        </div>
      </main>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()

// 设置状态
const settings = ref({
  // 常规设置
  theme: 'auto',
  language: 'zh-CN',
  startMinimized: false,
  autoCheckUpdates: true,
  showWelcome: true,

  // 解压设置
  defaultOutputPath: '',
  defaultKeepStructure: true,
  defaultOverwrite: false,
  defaultDeleteAfter: false,
  maxConcurrentTasks: 4,

  // 安全设置
  scanForViruses: true,
  checkFileExtensions: true,
  warnLargeFiles: true,
  savePasswords: false,
  encryptPasswords: true,
  autoClearPasswords: true,
  collectUsageData: false,
  sendCrashReports: true,

  // 高级设置
  cacheSize: 200,
  logLevel: 'info'
})

// 方法
const saveSettings = () => {
  // 这里应该调用API保存设置
  console.log('保存设置:', settings.value)
  alert('设置已保存！')
  router.back()
}

const cancel = () => {
  router.back()
}

const resetSettings = () => {
  if (confirm('确定要重置所有设置为默认值吗？')) {
    // 重置为默认值
    settings.value = {
      theme: 'auto',
      language: 'zh-CN',
      startMinimized: false,
      autoCheckUpdates: true,
      showWelcome: true,
      defaultOutputPath: '',
      defaultKeepStructure: true,
      defaultOverwrite: false,
      defaultDeleteAfter: false,
      maxConcurrentTasks: 4,
      scanForViruses: true,
      checkFileExtensions: true,
      warnLargeFiles: true,
      savePasswords: false,
      encryptPasswords: true,
      autoClearPasswords: true,
      collectUsageData: false,
      sendCrashReports: true,
      cacheSize: 200,
      logLevel: 'info'
    }
    alert('设置已重置为默认值')
  }
}

const clearCache = () => {
  if (confirm('确定要清除所有缓存文件吗？')) {
    // 这里应该调用API清除缓存
    console.log('清除缓存')
    alert('缓存已清除！')
  }
}
</script>

<style scoped>
input[type="checkbox"], input[type="range"], select {
  @apply rounded border-gray-300 text-primary focus:ring-primary;
}

input[type="range"] {
  @apply h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer;
}

input[type="range"]::-webkit-slider-thumb {
  @apply appearance-none w-4 h-4 rounded-full bg-primary cursor-pointer;
}
</style>